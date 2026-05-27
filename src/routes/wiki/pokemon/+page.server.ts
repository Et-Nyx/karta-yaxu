import { db } from '$lib/db';
import { eq, asc, desc, sql } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { sortLinkedListEntries } from '$lib/editor/pokedex';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  // 1. Fetch all available pokedexes
  const pokedexesList = await db.select({
    id: schema.pokedexes.id,
    name: schema.pokedexes.name,
    slug: schema.pokedexes.slug,
    source: schema.pokedexes.source,
  })
  .from(schema.pokedexes)
  .orderBy(asc(schema.pokedexes.name));

  // 2. Fetch entries for all pokedexes
  const rawEntries = await db.select({
    pokedexId: schema.pokedexEntries.pokedexId,
    pokemonId: schema.pokedexEntries.pokemonId,
    entryNumber: schema.pokedexEntries.entryNumber,
    nextPokemonId: schema.pokedexEntries.nextPokemonId,
  })
  .from(schema.pokedexEntries);

  const pokedexEntriesMap: Record<number, number[]> = {};

  for (const dex of pokedexesList) {
    const entries = rawEntries.filter(e => e.pokedexId === dex.id);
    if (entries.length === 0) {
      pokedexEntriesMap[dex.id] = [];
      continue;
    }

    let sorted = [];
    if (dex.source === 'custom') {
      sorted = sortLinkedListEntries(entries);
    } else {
      sorted = [...entries].sort((a, b) => a.entryNumber - b.entryNumber);
    }

    pokedexEntriesMap[dex.id] = sorted.map(s => s.pokemonId);
  }

  // 3. Fetch all pokemon (including regional forms), ordered by Regional Dex.
  // Within a species: default form first, then regional variants alphabetically.
  const isDefaultExpr = sql<boolean>`(${schema.pokemon.id} = ${schema.pokemonSpecies.defaultPokemonId})`;
  const pokemonList = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
    isDefault: isDefaultExpr,
    dexNumber: schema.pokemonSpecies.dexNumber,
  })
  .from(schema.pokemon)
  .innerJoin(schema.pokemonSpecies, eq(schema.pokemon.speciesId, schema.pokemonSpecies.id))
  .orderBy(asc(schema.pokemonSpecies.dexOrder), desc(isDefaultExpr), asc(schema.pokemon.name))
  .limit(2000);

  // 4. Fetch all types for these pokemon
  const pokemonTypesList = await db.select({
    pokemonId: schema.pokemonTypes.pokemonId,
    typeName: schema.types.name,
    typeHexColor: schema.types.hexColor,
    slot: schema.pokemonTypes.slot,
  })
  .from(schema.pokemonTypes)
  .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
  .orderBy(asc(schema.pokemonTypes.slot));

  // 5. Merge in memory
  const typesMap = new Map<number, Array<{ name: string; hexColor: string | null }>>();
  for (const pt of pokemonTypesList) {
    if (!typesMap.has(pt.pokemonId)) {
      typesMap.set(pt.pokemonId, []);
    }
    typesMap.get(pt.pokemonId)!.push({ name: pt.typeName, hexColor: pt.typeHexColor });
  }

  const mergedPokemon = pokemonList.map(p => ({
    ...p,
    types: typesMap.get(p.id) || []
  }));

  return {
    pokemon: mergedPokemon,
    pokedexes: pokedexesList,
    pokedexEntriesMap
  };
};
