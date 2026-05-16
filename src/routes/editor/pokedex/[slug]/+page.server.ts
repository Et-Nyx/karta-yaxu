import { error, fail, redirect } from '@sveltejs/kit';
import { and, asc, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { pokedexes, regions } from '$lib/db/schema/game-metadata';
import { pokedexEntries, pokemonSpecies } from '$lib/db/schema/pokemon';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const UpdateSchema = z.object({
  name:         z.string().trim().min(1).max(80),
  slug:         z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  regionId:     z.preprocess(
    (v) => v === '' || v == null ? null : Number(v),
    z.number().int().positive().nullable(),
  ),
  isMainSeries: z.preprocess((v) => v === 'true' || v === 'on' || v === true, z.boolean()),
});

const EntrySchema = z.object({
  speciesId:   z.coerce.number().int().positive(),
  entryNumber: z.coerce.number().int().min(0).max(9999),
});

const UpdateEntrySchema = z.object({
  speciesId:   z.coerce.number().int().positive(),
  entryNumber: z.coerce.number().int().min(0).max(9999),
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Pokédex not found');

  const entries = await db.select({
    speciesId:    pokedexEntries.speciesId,
    entryNumber:  pokedexEntries.entryNumber,
    speciesName:  pokemonSpecies.name,
    speciesSlug:  pokemonSpecies.slug,
  })
    .from(pokedexEntries)
    .innerJoin(pokemonSpecies, eq(pokedexEntries.speciesId, pokemonSpecies.id))
    .where(eq(pokedexEntries.pokedexId, row.id))
    .orderBy(asc(pokedexEntries.entryNumber));

  const speciesList = await db.select({
    id:   pokemonSpecies.id,
    name: pokemonSpecies.name,
    slug: pokemonSpecies.slug,
  }).from(pokemonSpecies).orderBy(asc(pokemonSpecies.dexNumber));

  const regionList = await db.select({
    id:   regions.id,
    name: regions.name,
    slug: regions.slug,
  }).from(regions).orderBy(asc(regions.id));

  return { pokedex: row, entries, species: speciesList, regions: regionList };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { updateError: parsed.error.issues[0]?.message ?? 'Inválido', values: form });
    }
    const data = parsed.data;

    const [current] = await db.select().from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: pokedexes.id })
        .from(pokedexes).where(eq(pokedexes.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:         data.name,
      slug:         data.slug,
      regionId:     data.regionId,
      isMainSeries: data.isMainSeries,
      source:       bumpSource(current.source),
    };

    await db.update(pokedexes).set(newRow).where(eq(pokedexes.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedexes', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/pokedex/${data.slug}`);
    return { saved: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas pokédexes custom podem ser deletadas' });
    }

    await db.delete(pokedexEntries).where(eq(pokedexEntries.pokedexId, current.id));
    await db.delete(pokedexes).where(eq(pokedexes.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedexes', current.id, current, null);
    }

    throw redirect(303, '/editor/pokedex');
  },

  addEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = EntrySchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { entryError: parsed.error.issues[0]?.message ?? 'Entrada inválida' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokedexes.id })
      .from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');

    const [exists] = await db.select({ speciesId: pokedexEntries.speciesId })
      .from(pokedexEntries)
      .where(and(
        eq(pokedexEntries.pokedexId, current.id),
        eq(pokedexEntries.speciesId, data.speciesId),
      )).limit(1);
    if (exists) return fail(400, { entryError: 'Essa species já está na pokédex' });

    await db.insert(pokedexEntries).values({
      pokedexId:   current.id,
      speciesId:   data.speciesId,
      entryNumber: data.entryNumber,
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedex_entries', current.id, null, data);
    }

    return { entryOk: true };
  },

  removeEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const speciesId = Number(form.speciesId);
    if (!Number.isInteger(speciesId) || speciesId <= 0) {
      return fail(400, { entryError: 'speciesId inválido' });
    }

    const [current] = await db.select({ id: pokedexes.id })
      .from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');

    const [existing] = await db.select()
      .from(pokedexEntries)
      .where(and(
        eq(pokedexEntries.pokedexId, current.id),
        eq(pokedexEntries.speciesId, speciesId),
      )).limit(1);
    if (!existing) return fail(404, { entryError: 'Entrada não encontrada' });

    await db.delete(pokedexEntries).where(and(
      eq(pokedexEntries.pokedexId, current.id),
      eq(pokedexEntries.speciesId, speciesId),
    ));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedex_entries', current.id, existing, null);
    }

    return { entryOk: true };
  },

  updateEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateEntrySchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { entryError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokedexes.id })
      .from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');

    const [existing] = await db.select()
      .from(pokedexEntries)
      .where(and(
        eq(pokedexEntries.pokedexId, current.id),
        eq(pokedexEntries.speciesId, data.speciesId),
      )).limit(1);
    if (!existing) return fail(404, { entryError: 'Entrada não encontrada' });

    await db.update(pokedexEntries)
      .set({ entryNumber: data.entryNumber })
      .where(and(
        eq(pokedexEntries.pokedexId, current.id),
        eq(pokedexEntries.speciesId, data.speciesId),
      ));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedex_entries', current.id, existing, { ...existing, entryNumber: data.entryNumber });
    }

    return { entryOk: true };
  },
};
