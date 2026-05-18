import { error, fail, redirect } from '@sveltejs/kit';
import { eq, and } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import { asc } from 'drizzle-orm';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const optionalInt = z
  .union([z.literal(''), z.coerce.number().int().min(0)])
  .optional()
  .transform((v) => (v === '' || v === undefined ? undefined : v));

const optionalText = z
  .string()
  .trim()
  .optional()
  .transform((v) => (v === '' || v === undefined ? undefined : v));

const UpdateSchema = z.object({
  name:       z.string().trim().min(1).max(100),
  slug:       z.string().trim().min(1).max(100).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  cost:       optionalInt,
  flingPower: optionalInt,
  category:   optionalText,
  spriteUrl:  optionalText,
});

const UpdateEffectsSchema = z.object({
  effect:      z.string().trim().min(1),
  shortEffect: z.string().trim().min(1),
});

export const load: PageServerLoad = async ({ params }) => {
  const iList = await db.select()
    .from(schema.items)
    .where(eq(schema.items.slug, params.slug))
    .limit(1);

  if (iList.length === 0) throw error(404, 'Item not found');
  const item = iList[0];

  // English language ID
  const langList = await db.select({ id: schema.languages.id })
    .from(schema.languages)
    .where(eq(schema.languages.name, 'en'))
    .limit(1);

  const englishId = langList[0]?.id ?? null;

  // Flavor texts with version group name
  const flavorTexts = englishId
    ? await db.select({
        flavorText: schema.itemFlavorTexts.flavorText,
        versionGroupId: schema.itemFlavorTexts.versionGroupId,
        versionGroupName: schema.versionGroups.name,
      })
      .from(schema.itemFlavorTexts)
      .innerJoin(schema.versionGroups, eq(schema.itemFlavorTexts.versionGroupId, schema.versionGroups.id))
      .where(and(eq(schema.itemFlavorTexts.itemId, item.id), eq(schema.itemFlavorTexts.languageId, englishId)))
      .orderBy(asc(schema.versionGroups.order))
    : [];

  // Effect (English)
  const effectList = englishId
    ? await db.select({
        effect: schema.itemEffects.effect,
        shortEffect: schema.itemEffects.shortEffect,
      })
      .from(schema.itemEffects)
      .where(and(eq(schema.itemEffects.itemId, item.id), eq(schema.itemEffects.languageId, englishId)))
      .limit(1)
    : [];

  const effect = effectList[0] ?? null;

  return { item, flavorTexts, effect };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, {
        updateError: parsed.error.issues[0]?.message ?? 'Inválido',
        values: form,
      });
    }

    const data = parsed.data;
    const [current] = await db.select().from(schema.items).where(eq(schema.items.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Item not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: schema.items.id })
        .from(schema.items).where(eq(schema.items.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:       data.name,
      slug:       data.slug,
      cost:       data.cost ?? null,
      flingPower: data.flingPower ?? null,
      category:   data.category ?? null,
      spriteUrl:  data.spriteUrl ?? null,
      source:     bumpSource(current.source),
    };

    await db.update(schema.items).set(newRow).where(eq(schema.items.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/items/${data.slug}`);
    return { saved: true };
  },

  updateEffects: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateEffectsSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, {
        updateError: parsed.error.issues[0]?.message ?? 'Inválido',
        values: form,
      });
    }

    const [item] = await db.select().from(schema.items).where(eq(schema.items.slug, params.slug)).limit(1);
    if (!item) throw error(404, 'Item not found');

    const langList = await db.select({ id: schema.languages.id })
      .from(schema.languages)
      .where(eq(schema.languages.name, 'en'))
      .limit(1);
    const englishId = langList[0]?.id;
    if (!englishId) return fail(500, { updateError: 'English language not found' });

    const existing = await db.select()
      .from(schema.itemEffects)
      .where(and(eq(schema.itemEffects.itemId, item.id), eq(schema.itemEffects.languageId, englishId)))
      .limit(1);

    if (existing.length > 0) {
      await db.update(schema.itemEffects)
        .set({ effect: parsed.data.effect, shortEffect: parsed.data.shortEffect })
        .where(and(eq(schema.itemEffects.itemId, item.id), eq(schema.itemEffects.languageId, englishId)));
    } else {
      await db.insert(schema.itemEffects)
        .values({ itemId: item.id, languageId: englishId, effect: parsed.data.effect, shortEffect: parsed.data.shortEffect });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'item_effects', item.id, null, parsed.data);
    }

    return { savedEffects: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(schema.items).where(eq(schema.items.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Item not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(schema.items).set({ source: 'pokeapi' }).where(eq(schema.items.id, current.id));
    try {
      await reimportEntity('items', current.pokeapiId);
    } catch (err) {
      await db.update(schema.items).set({ source: 'edited' }).where(eq(schema.items.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(schema.items).where(eq(schema.items.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Item not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas registros source=custom podem ser deletados' });
    }

    await db.delete(schema.items).where(eq(schema.items.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', current.id, current, null);
    }

    throw redirect(303, '/editor/items');
  },
};
