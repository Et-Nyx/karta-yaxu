import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { pokedexes, regions } from '$lib/db/schema/game-metadata';
import { pokedexEntries } from '$lib/db/schema/pokemon';
import { recordEdit } from '$lib/editor/audit';
import { copyStaticPokedex } from '$lib/editor/pokedex';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const CreateSchema = z.object({
  name:         z.string().trim().min(1).max(80),
  slug:         z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letters/numbers/hyphen'),
  regionId:     z.preprocess(
    (v) => v === '' || v == null ? null : Number(v),
    z.number().int().positive().nullable(),
  ),
  copyFromId:   z.preprocess(
    (v) => v === '' || v == null ? null : Number(v),
    z.number().int().positive().nullable(),
  ),
  isMainSeries: z.preprocess((v) => v === 'true' || v === 'on' || v === true, z.boolean()),
});

export const load: PageServerLoad = async () => {
  const rows = await db.select({
    id:           pokedexes.id,
    name:         pokedexes.name,
    slug:         pokedexes.slug,
    regionId:     pokedexes.regionId,
    isMainSeries: pokedexes.isMainSeries,
    source:       pokedexes.source,
    pokeapiId:    pokedexes.pokeapiId,
    entryCount:   count(pokedexEntries.speciesId),
  })
    .from(pokedexes)
    .leftJoin(pokedexEntries, eq(pokedexes.id, pokedexEntries.pokedexId))
    .groupBy(pokedexes.id)
    .orderBy(asc(pokedexes.id));

  const regionList = await db.select({
    id:   regions.id,
    name: regions.name,
    slug: regions.slug,
  }).from(regions).orderBy(asc(regions.id));

  const [{ total }] = await db.select({ total: count() }).from(pokedexes);

  return { pokedexes: rows, regions: regionList, total };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = CreateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { createError: parsed.error.issues[0]?.message ?? 'Invalid', createValues: form });
    }
    const data = parsed.data;

    const [existing] = await db.select({ id: pokedexes.id })
      .from(pokedexes).where(eq(pokedexes.slug, data.slug)).limit(1);
    if (existing) return fail(400, { createError: 'Slug already in use', createValues: form });

    let insertedId: number;
    if (data.copyFromId) {
      insertedId = await copyStaticPokedex(data.copyFromId, data.name, data.slug, data.regionId);
    } else {
      const [inserted] = await db.insert(pokedexes).values({
        name:         data.name,
        slug:         data.slug,
        regionId:     data.regionId,
        isMainSeries: data.isMainSeries,
        source:       'custom',
        pokeapiId:    null,
      }).returning({ id: pokedexes.id });
      insertedId = inserted.id;
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedexes', insertedId, null, data);
    }

    throw redirect(303, `/editor/pokedex/${data.slug}`);
  },
};
