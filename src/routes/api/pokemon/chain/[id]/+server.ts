import { json } from '@sveltejs/kit';
import { and, eq, inArray, asc } from 'drizzle-orm';
import { db } from '$lib/db';
import { pokemon, pokemonSpecies, pokemonTypes } from '$lib/db/schema/pokemon';
import { types } from '$lib/db/schema/core-data';
import { getEvolutionChainSpecies } from '$lib/editor/evolution';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ params }) => {
  const pokemonId = Number(params.id);
  
  // 1. Get speciesId for this pokemon form
  const [pk] = await db.select({ speciesId: pokemon.speciesId })
    .from(pokemon)
    .where(eq(pokemon.id, pokemonId))
    .limit(1);

  if (!pk) {
    return new Response(JSON.stringify({ error: 'Pokemon not found' }), { status: 404 });
  }

  // 2. Fetch evolution chain species ids
  const chainSpeciesIds = await getEvolutionChainSpecies(pk.speciesId);
  if (chainSpeciesIds.length === 0) {
    return json([]);
  }

  // 3. Fetch default pokemon variants for each species in the chain
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
    .where(
      and(
        inArray(pokemon.speciesId, chainSpeciesIds),
        eq(pokemon.id, pokemonSpecies.defaultPokemonId)
      )
    );

  // Fallback if defaultPokemonId is not set
  const resolvedPokemon: typeof pkList = [];
  for (const sId of chainSpeciesIds) {
    const matched = pkList.find(p => p.speciesId === sId);
    if (matched) {
      resolvedPokemon.push(matched);
    } else {
      const [fallback] = await db.select({
        pokemonId: pokemon.id,
        speciesId: pokemon.speciesId,
        pokemonName: pokemon.name,
        pokemonSlug: pokemon.slug,
        spriteDefault: pokemon.spriteDefault,
        pokeapiId: pokemonSpecies.pokeapiId,
      })
        .from(pokemon)
        .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
        .where(eq(pokemon.speciesId, sId))
        .limit(1);
      if (fallback) {
        resolvedPokemon.push(fallback);
      }
    }
  }

  // 4. Fetch types for all resolved pokemon
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

  // 5. Build standard entries mapping
  const entries = resolvedPokemon.map((p, index) => {
    const chainId = Math.min(...chainSpeciesIds);
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
