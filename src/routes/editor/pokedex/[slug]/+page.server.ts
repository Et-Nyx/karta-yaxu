import { error, fail, redirect } from '@sveltejs/kit';
import { and, asc, eq, inArray } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { pokedexes, regions } from '$lib/db/schema/game-metadata';
import { pokedexEntries, pokemonSpecies, pokemon, pokemonTypes } from '$lib/db/schema/pokemon';
import { types } from '$lib/db/schema/core-data';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { getEvolutionChainSpecies } from '$lib/editor/evolution';
import {
  sortLinkedListEntries,
  savePokedexOrder,
  addPokemonToPokedex,
  removePokemonFromPokedex,
  copyStaticPokedex,
  syncCustomPokedexEntries
} from '$lib/editor/pokedex';
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

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Pokédex not found');

  // Automatically sync custom pokedex entries with any recent evolution changes
  if (row.source === 'custom') {
    await syncCustomPokedexEntries(row.id);
  }

  // Fetch all flat entries and join the pokemon and species details
  const entriesRaw = await db.select({
    pokemonId:        pokedexEntries.pokemonId,
    speciesId:        pokedexEntries.speciesId,
    entryNumber:      pokedexEntries.entryNumber,
    nextPokemonId:    pokedexEntries.nextPokemonId,
    nextSpeciesId:    pokedexEntries.nextSpeciesId,
    pokemonName:      pokemon.name,
    pokemonSlug:      pokemon.slug,
    pokeapiId:        pokemon.pokeapiId,
    spriteDefault:    pokemon.spriteDefault,
  })
    .from(pokedexEntries)
    .innerJoin(pokemon, eq(pokedexEntries.pokemonId, pokemon.id))
    .innerJoin(pokemonSpecies, eq(pokedexEntries.speciesId, pokemonSpecies.id))
    .where(eq(pokedexEntries.pokedexId, row.id));

  // Sort custom linked-list Pokédexes using our helper, otherwise sort static by entryNumber
  let sortedEntriesRaw = [];
  if (row.source === 'custom') {
    sortedEntriesRaw = sortLinkedListEntries(entriesRaw);
  } else {
    sortedEntriesRaw = [...entriesRaw].sort((a, b) => a.entryNumber - b.entryNumber);
  }

  // Fetch the Pokémon types in bulk queries
  const pokemonIds = sortedEntriesRaw.map(e => e.pokemonId);
  
  let pokemonTypesMap = new Map<number, any[]>();

  if (pokemonIds.length > 0) {
    const typesData = await db.select({
      pokemonId:    pokemonTypes.pokemonId,
      typeName:     types.name,
      typeHexColor: types.hexColor,
    })
      .from(pokemonTypes)
      .innerJoin(types, eq(pokemonTypes.typeId, types.id))
      .where(inArray(pokemonTypes.pokemonId, pokemonIds))
      .orderBy(asc(pokemonTypes.slot));

    for (const t of typesData) {
      if (!pokemonTypesMap.has(t.pokemonId)) {
        pokemonTypesMap.set(t.pokemonId, []);
      }
      pokemonTypesMap.get(t.pokemonId)!.push({ name: t.typeName, hexColor: t.typeHexColor });
    }
  }

  const entries = await Promise.all(sortedEntriesRaw.map(async (e, index) => {
    const calculatedNumber = index + 1;
    const chainSpeciesIds = await getEvolutionChainSpecies(e.speciesId);
    const chainId = Math.min(...chainSpeciesIds);
    return {
      pokemonId:   e.pokemonId,
      speciesId:   e.speciesId,
      chainId,
      entryNumber: calculatedNumber,
      pokemonName: e.pokemonName,
      pokemonSlug: e.pokemonSlug,
      pokemonCard: {
        pokeapiId: e.pokeapiId ?? 0,
        name:      e.pokemonName,
        slug:      e.pokemonSlug,
        spriteUrl: e.spriteDefault ?? null,
        types:     pokemonTypesMap.get(e.pokemonId) ?? [],
        dexNumber: calculatedNumber,
      }
    };
  }));

  const pokemonList = await db.select({
    id:   pokemon.id,
    name: pokemon.name,
    slug: pokemon.slug,
  }).from(pokemon).orderBy(asc(pokemon.id));

  const regionList = await db.select({
    id:   regions.id,
    name: regions.name,
    slug: regions.slug,
  }).from(regions).orderBy(asc(regions.id));

  return { pokedex: row, entries, pokemonList, regions: regionList };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { updateError: parsed.error.issues[0]?.message ?? 'Invalid', values: form });
    }
    const data = parsed.data;

    const [current] = await db.select().from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Only custom Pokédexes can be edited', values: form });
    }

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: pokedexes.id })
        .from(pokedexes).where(eq(pokedexes.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'Slug already in use', values: form });
    }

    const newRow = {
      name:         data.name,
      slug:         data.slug,
      regionId:     data.regionId,
      isMainSeries: data.isMainSeries,
      source:       bumpSource(current.source),
    };

    await db.update(pokedexes).set(newRow).where(eq(pokedexes.id, current.id));

    if (form.pokemonIds && typeof form.pokemonIds === 'string') {
      const pokemonIds = form.pokemonIds.split(',').map(Number);
      if (!pokemonIds.some(isNaN)) {
        await savePokedexOrder(current.id, pokemonIds);
      }
    }

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
      return fail(400, { updateError: 'Only custom Pokédexes can be deleted' });
    }

    await db.delete(pokedexEntries).where(eq(pokedexEntries.pokedexId, current.id));
    await db.delete(pokedexes).where(eq(pokedexes.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedexes', current.id, current, null);
    }

    throw redirect(303, '/editor/pokedex');
  },

  copy: async ({ params, locals }) => {
    const [current] = await db.select().from(pokedexes).where(eq(pokedexes.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokédex not found');

    // Create a new slug and name in English
    let baseSlug = `${current.slug}-copy`;
    let newSlug = baseSlug;
    let counter = 1;

    // Check slug clashes
    while (true) {
      const [clash] = await db.select({ id: pokedexes.id })
        .from(pokedexes).where(eq(pokedexes.slug, newSlug)).limit(1);
      if (!clash) break;
      newSlug = `${baseSlug}-${counter++}`;
    }

    const newName = newSlug;
    const newPokedexId = await copyStaticPokedex(current.id, newName, newSlug, current.regionId);

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokedexes', newPokedexId, null, { id: newPokedexId, name: newName, slug: newSlug });
    }

    throw redirect(303, `/editor/pokedex/${newSlug}`);
  },
};
