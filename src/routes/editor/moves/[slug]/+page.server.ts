import { error, fail, redirect } from '@sveltejs/kit';
import { and, asc, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { moves, moveEffects, types } from '$lib/db/schema/core-data';
import { generations, languages } from '$lib/db/schema/game-metadata';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const intOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
  z.number().int().min(0).nullable(),
);

const UpdateSchema = z.object({
  name:         z.string().trim().min(1).max(80),
  slug:         z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  typeId:       z.coerce.number().int().positive(),
  damageClass:  z.enum(['physical', 'special', 'status']),
  power:        intOrNull,
  accuracy:     z.preprocess(
    (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
    z.number().int().min(0).max(100).nullable(),
  ),
  pp:           z.coerce.number().int().min(0),
  priority:     z.coerce.number().int().default(0),
  target:       z.preprocess(
    (v) => (v === '' || v === null || v === undefined ? null : String(v)),
    z.string().nullable(),
  ),
  generationId: intOrNull,
});

const EffectsSchema = z.object({
  effect:      z.string().trim().min(0).max(4000),
  shortEffect: z.string().trim().min(0).max(2000),
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(moves).where(eq(moves.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Move not found');

  const [enLang] = await db.select({ id: languages.id })
    .from(languages).where(eq(languages.name, 'en')).limit(1);

  let effects: { effect: string; shortEffect: string } | null = null;
  if (enLang) {
    const [eff] = await db.select({
      effect:      moveEffects.effect,
      shortEffect: moveEffects.shortEffect,
    })
      .from(moveEffects)
      .where(and(
        eq(moveEffects.moveId, row.id),
        eq(moveEffects.languageId, enLang.id),
      )).limit(1);
    if (eff) effects = eff;
  }

  const typeList = await db.select({
    id:   types.id,
    name: types.name,
    slug: types.slug,
  }).from(types).orderBy(asc(types.id));

  const generationList = await db.select({
    id:   generations.id,
    name: generations.name,
    slug: generations.slug,
  }).from(generations).orderBy(asc(generations.id));

  return {
    move:          row,
    effects,
    types:         typeList,
    generations:   generationList,
    englishLangId: enLang?.id ?? null,
  };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { updateError: parsed.error.issues[0]?.message ?? 'Inválido', values: form });
    }
    const data = parsed.data;

    const [current] = await db.select().from(moves).where(eq(moves.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Move not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: moves.id })
        .from(moves).where(eq(moves.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:         data.name,
      slug:         data.slug,
      typeId:       data.typeId,
      damageClass:  data.damageClass,
      power:        data.power,
      accuracy:     data.accuracy,
      pp:           data.pp,
      priority:     data.priority,
      target:       data.target,
      generationId: data.generationId,
      source:       bumpSource(current.source),
    };

    await db.update(moves).set(newRow).where(eq(moves.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'moves', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/moves/${data.slug}`);
    return { saved: true };
  },

  updateEffects: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = EffectsSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { effectsError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: moves.id })
      .from(moves).where(eq(moves.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Move not found');

    const [enLang] = await db.select({ id: languages.id })
      .from(languages).where(eq(languages.name, 'en')).limit(1);
    if (!enLang) return fail(400, { effectsError: 'English language row not found' });

    const [previous] = await db.select({
      effect:      moveEffects.effect,
      shortEffect: moveEffects.shortEffect,
    })
      .from(moveEffects)
      .where(and(
        eq(moveEffects.moveId, current.id),
        eq(moveEffects.languageId, enLang.id),
      )).limit(1);

    await db.insert(moveEffects).values({
      moveId:      current.id,
      languageId:  enLang.id,
      effect:      data.effect,
      shortEffect: data.shortEffect,
    }).onConflictDoUpdate({
      target: [moveEffects.moveId, moveEffects.languageId],
      set:    { effect: data.effect, shortEffect: data.shortEffect },
    });

    if (locals.user) {
      await recordEdit(
        locals.user.id,
        'move_effects',
        current.id,
        previous ?? null,
        { effect: data.effect, shortEffect: data.shortEffect },
      );
    }

    return { effectsSaved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(moves).where(eq(moves.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Move not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(moves).set({ source: 'pokeapi' }).where(eq(moves.id, current.id));
    try {
      await reimportEntity('moves', current.pokeapiId);
    } catch (err) {
      await db.update(moves).set({ source: 'edited' }).where(eq(moves.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'moves', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(moves).where(eq(moves.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Move not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas moves custom podem ser deletados' });
    }

    await db.delete(moveEffects).where(eq(moveEffects.moveId, current.id));
    await db.delete(moves).where(eq(moves.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'moves', current.id, current, null);
    }

    throw redirect(303, '/editor/moves');
  },
};
