import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { moves, types } from '$lib/db/schema/core-data';
import { generations } from '$lib/db/schema/game-metadata';
import { recordEdit } from '$lib/editor/audit';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const intOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
  z.number().int().min(0).nullable(),
);

const CreateSchema = z.object({
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

export const load: PageServerLoad = async () => {
  const rows = await db.select({
    id:          moves.id,
    name:        moves.name,
    slug:        moves.slug,
    typeId:      moves.typeId,
    damageClass: moves.damageClass,
    power:       moves.power,
    accuracy:    moves.accuracy,
    pp:          moves.pp,
    source:      moves.source,
    pokeapiId:   moves.pokeapiId,
  }).from(moves).orderBy(asc(moves.slug));

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

  const [{ total }] = await db.select({ total: count() }).from(moves);

  return { moves: rows, types: typeList, generations: generationList, total };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = CreateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { createError: parsed.error.issues[0]?.message ?? 'Inválido', createValues: form });
    }
    const data = parsed.data;

    const [existing] = await db.select({ id: moves.id })
      .from(moves).where(eq(moves.slug, data.slug)).limit(1);
    if (existing) return fail(400, { createError: 'slug já em uso', createValues: form });

    const [inserted] = await db.insert(moves).values({
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
      source:       'custom',
      pokeapiId:    null,
    }).returning({ id: moves.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'moves', inserted.id, null, data);
    }

    throw redirect(303, `/editor/moves/${data.slug}`);
  },
};
