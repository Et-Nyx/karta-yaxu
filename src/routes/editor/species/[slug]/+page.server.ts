import { error, fail, redirect } from '@sveltejs/kit';
import { asc, eq, inArray } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import {
  eggGroups,
  pokemon,
  pokemonAbilities,
  pokemonForms,
  pokemonMoves,
  pokemonSpecies,
  pokemonTypes,
  speciesEggGroups,
} from '$lib/db/schema/pokemon';
import { generations } from '$lib/db/schema/game-metadata';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const intOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
  z.number().int().nullable(),
);

const textOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : String(v)),
  z.string().nullable(),
);

const UpdateSchema = z.object({
  name:          z.string().trim().min(1).max(80),
  slug:          z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  genus:         textOrNull,
  dexNumber:     z.coerce.number().int().min(0).max(9999),
  isBaby:        boolField,
  isLegendary:   boolField,
  isMythical:    boolField,
  habitat:       textOrNull,
  color:         textOrNull,
  shape:         textOrNull,
  growthRate:    textOrNull,
  baseHappiness: intOrNull,
  captureRate:   intOrNull,
  genderRate:    intOrNull,
  hatchCounter:  intOrNull,
  generationId:  intOrNull,
  order:         intOrNull,
  dexOrder:      intOrNull,
  defaultPokemonId: intOrNull,
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Species not found');

  const generationList = await db.select({
    id:   generations.id,
    name: generations.name,
    slug: generations.slug,
  }).from(generations).orderBy(asc(generations.id));

  const pokemonRows = (await db.select({
    id:        pokemon.id,
    slug:      pokemon.slug,
    name:      pokemon.name,
    source:    pokemon.source,
  }).from(pokemon).where(eq(pokemon.speciesId, row.id)).orderBy(asc(pokemon.id))).map(p => ({
    ...p,
    isDefault: p.id === row.defaultPokemonId,
  }));

  const allEggGroups = await db.select({
    id:   eggGroups.id,
    name: eggGroups.name,
    slug: eggGroups.slug,
  }).from(eggGroups).orderBy(asc(eggGroups.id));

  const currentEggGroups = await db.select({
    eggGroupId: speciesEggGroups.eggGroupId,
    name:       eggGroups.name,
    slug:       eggGroups.slug,
  })
    .from(speciesEggGroups)
    .innerJoin(eggGroups, eq(speciesEggGroups.eggGroupId, eggGroups.id))
    .where(eq(speciesEggGroups.speciesId, row.id))
    .orderBy(asc(eggGroups.id));

  return {
    species: row,
    generations: generationList,
    pokemon: pokemonRows,
    allEggGroups,
    currentEggGroups,
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

    const [current] = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Species not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: pokemonSpecies.id })
        .from(pokemonSpecies).where(eq(pokemonSpecies.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:          data.name,
      slug:          data.slug,
      genus:         data.genus,
      dexNumber:     data.dexNumber,
      isBaby:        data.isBaby,
      isLegendary:   data.isLegendary,
      isMythical:    data.isMythical,
      habitat:       data.habitat,
      color:         data.color,
      shape:         data.shape,
      growthRate:    data.growthRate,
      baseHappiness: data.baseHappiness,
      captureRate:   data.captureRate,
      genderRate:    data.genderRate,
      hatchCounter:  data.hatchCounter,
      generationId:  data.generationId,
      order:         data.order,
      dexOrder:      data.dexOrder,
      defaultPokemonId: data.defaultPokemonId,
      source:        bumpSource(current.source),
    };

    await db.update(pokemonSpecies).set(newRow).where(eq(pokemonSpecies.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_species', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/species/${data.slug}`);
    return { saved: true };
  },

  updateEggGroups: async ({ request, params, locals }) => {
    const formData = await request.formData();
    const rawIds = formData.getAll('eggGroupIds');

    const ids: number[] = [];
    for (const raw of rawIds) {
      const n = Number(raw);
      if (!Number.isInteger(n) || n <= 0) {
        return fail(400, { eggGroupsError: `eggGroupId inválido: ${String(raw)}` });
      }
      if (!ids.includes(n)) ids.push(n);
    }

    const [current] = await db.select({ id: pokemonSpecies.id })
      .from(pokemonSpecies).where(eq(pokemonSpecies.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Species not found');

    if (ids.length > 0) {
      const existing = await db.select({ id: eggGroups.id })
        .from(eggGroups).where(inArray(eggGroups.id, ids));
      const existingSet = new Set(existing.map(e => e.id));
      const missing = ids.filter(i => !existingSet.has(i));
      if (missing.length > 0) {
        return fail(400, { eggGroupsError: `egg group inexistente: ${missing.join(', ')}` });
      }
    }

    const previousLinks = await db.select({
      speciesId:  speciesEggGroups.speciesId,
      eggGroupId: speciesEggGroups.eggGroupId,
    }).from(speciesEggGroups).where(eq(speciesEggGroups.speciesId, current.id));

    await db.delete(speciesEggGroups).where(eq(speciesEggGroups.speciesId, current.id));
    if (ids.length > 0) {
      await db.insert(speciesEggGroups).values(
        ids.map(eggGroupId => ({ speciesId: current.id, eggGroupId })),
      );
    }

    const newLinks = ids.map(eggGroupId => ({ speciesId: current.id, eggGroupId }));

    if (locals.user) {
      await recordEdit(locals.user.id, 'species_egg_groups', current.id, previousLinks, newLinks);
    }

    return { eggGroupsSaved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Species not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas species edited podem ser re-sourceadas' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(pokemonSpecies).set({ source: 'pokeapi' }).where(eq(pokemonSpecies.id, current.id));
    try {
      await reimportEntity('pokemon_species', current.pokeapiId);
    } catch (err) {
      await db.update(pokemonSpecies).set({ source: 'edited' }).where(eq(pokemonSpecies.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_species', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Species not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas species custom podem ser deletadas' });
    }

    // Gather all pokemon ids for this species (variants too)
    const linkedPokemon = await db.select({ id: pokemon.id })
      .from(pokemon).where(eq(pokemon.speciesId, current.id));
    const pokemonIds = linkedPokemon.map(p => p.id);

    db.transaction((tx) => {
      if (pokemonIds.length > 0) {
        tx.delete(pokemonMoves).where(inArray(pokemonMoves.pokemonId, pokemonIds)).run();
        tx.delete(pokemonTypes).where(inArray(pokemonTypes.pokemonId, pokemonIds)).run();
        tx.delete(pokemonAbilities).where(inArray(pokemonAbilities.pokemonId, pokemonIds)).run();
        tx.delete(pokemonForms).where(inArray(pokemonForms.pokemonId, pokemonIds)).run();
        tx.delete(pokemon).where(inArray(pokemon.id, pokemonIds)).run();
      }
      tx.delete(pokemonSpecies).where(eq(pokemonSpecies.id, current.id)).run();
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_species', current.id, current, null);
    }

    throw redirect(303, '/editor/species');
  },
};
