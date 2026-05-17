import { db } from '$lib/db';
import { eq, asc, desc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  // 1. Fetch all pokemon (including regional forms), ordered by Regional Dex.
  // Within a species: default form first, then regional variants alphabetically.
  const pokemonList = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
    isDefault: schema.pokemon.isDefault,
    dexNumber: schema.pokemonSpecies.dexNumber,
  })
  .from(schema.pokemon)
  .innerJoin(schema.pokemonSpecies, eq(schema.pokemon.speciesId, schema.pokemonSpecies.id))
  .orderBy(asc(schema.pokemonSpecies.dexOrder), desc(schema.pokemon.isDefault), asc(schema.pokemon.name))
  .limit(2000);

  // 2. Fetch all types for these pokemon
  const pokemonTypesList = await db.select({
    pokemonId: schema.pokemonTypes.pokemonId,
    typeName: schema.types.name,
    typeHexColor: schema.types.hexColor,
    slot: schema.pokemonTypes.slot,
  })
  .from(schema.pokemonTypes)
  .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
  .orderBy(asc(schema.pokemonTypes.slot));

  // 3. Merge in memory
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
    pokemon: mergedPokemon
  };
};
