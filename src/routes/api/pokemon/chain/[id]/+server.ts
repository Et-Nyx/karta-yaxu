import { json } from '@sveltejs/kit';
import { and, eq, inArray, asc } from 'drizzle-orm';
import { db } from '$lib/db';
import { pokemon, pokemonSpecies, pokemonTypes } from '$lib/db/schema/pokemon';
import { types } from '$lib/db/schema/core-data';
import { getEvolutionChainPokemon } from '$lib/editor/evolution';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params }) => {
  const pokemonId = Number(params.id);
  
  // 1. Fetch form-based evolution chain pokemon IDs
  const chainPokemonIds = await getEvolutionChainPokemon(pokemonId);
  if (chainPokemonIds.length === 0) {
    return json([]);
  }

  // 2. Fetch pokemon details for all resolved pokemon in the chain
  const pkList = await db.select({
    pokemonId: pokemon.id,
    speciesId: pokemon.speciesId,
    pokemonName: pokemon.name,
    pokemonSlug: pokemon.slug,
    spriteDefault: pokemon.spriteDefault,
    pokeapiId: pokemonSpecies.pokeapiId,
  })
    .from(pokemon)
    .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
    .where(inArray(pokemon.id, chainPokemonIds));

  // Sort them according to the topological chain order
  const resolvedPokemon: typeof pkList = [];
  for (const pId of chainPokemonIds) {
    const matched = pkList.find(p => p.pokemonId === pId);
    if (matched) resolvedPokemon.push(matched);
  }

  // 3. Fetch types for all resolved pokemon
  const pokemonIds = resolvedPokemon.map(p => p.pokemonId);
  const pokemonTypesMap = new Map<number, Array<{ name: string, hexColor: string }>>();
  
  if (pokemonIds.length > 0) {
    const typesData = await db.select({
      pokemonId: pokemonTypes.pokemonId,
      typeName: types.name,
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
      pokemonTypesMap.get(t.pokemonId)!.push({ name: t.typeName, hexColor: t.typeHexColor ?? '' });
    }
  }

  // 4. Build standard entries mapping
  const entries = resolvedPokemon.map((p, index) => {
    // Treat the root parent pokemon ID as the chainId group identifier
    const chainId = chainPokemonIds[0];
    return {
      pokemonId: p.pokemonId,
      speciesId: p.speciesId,
      chainId,
      entryNumber: index + 1,
      pokemonName: p.pokemonName,
      pokemonSlug: p.pokemonSlug,
      pokemonCard: {
        pokeapiId: p.pokeapiId ?? 0,
        name: p.pokemonName,
        slug: p.pokemonSlug,
        spriteUrl: p.spriteDefault ?? null,
        types: pokemonTypesMap.get(p.pokemonId) ?? [],
        dexNumber: index + 1,
      }
    };
  });

  return json(entries);
};
