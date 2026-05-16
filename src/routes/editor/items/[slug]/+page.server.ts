import { error, fail, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { items } from '$lib/db/schema/items';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
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

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(items).where(eq(items.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Item not found');
  return { item: row };
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
    const [current] = await db.select().from(items).where(eq(items.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Item not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: items.id })
        .from(items).where(eq(items.slug, data.slug)).limit(1);
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

    await db.update(items).set(newRow).where(eq(items.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/items/${data.slug}`);
    return { saved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(items).where(eq(items.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Item not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(items).set({ source: 'pokeapi' }).where(eq(items.id, current.id));
    try {
      await reimportEntity('items', current.pokeapiId);
    } catch (err) {
      await db.update(items).set({ source: 'edited' }).where(eq(items.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(items).where(eq(items.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Item not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas registros source=custom podem ser deletados' });
    }

    await db.delete(items).where(eq(items.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'items', current.id, current, null);
    }

    throw redirect(303, '/editor/items');
  },
};
