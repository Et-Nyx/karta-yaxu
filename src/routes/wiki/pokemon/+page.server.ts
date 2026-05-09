import { db } from '$lib/db';
import { desc, eq, asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  // 1. Fetch all default pokemon
  const pokemonList = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
    order: schema.pokemon.order,
  })
  .from(schema.pokemon)
  .where(eq(schema.pokemon.isDefault, true))
  .orderBy(asc(schema.pokemon.order))
  .limit(1010); // Adjust as needed

  // 2. Fetch all types for these pokemon
  const pokemonTypesList = await db.select({
    pokemonId: schema.pokemonTypes.pokemonId,
    typeName: schema.types.name,
    slot: schema.pokemonTypes.slot,
  })
  .from(schema.pokemonTypes)
  .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
  .orderBy(asc(schema.pokemonTypes.slot));

  // 3. Merge in memory
  const typesMap = new Map<number, string[]>();
  for (const pt of pokemonTypesList) {
    if (!typesMap.has(pt.pokemonId)) {
      typesMap.set(pt.pokemonId, []);
    }
    typesMap.get(pt.pokemonId)!.push(pt.typeName);
  }

  const mergedPokemon = pokemonList.map(p => ({
    ...p,
    types: typesMap.get(p.id) || []
  }));

  return {
    pokemon: mergedPokemon
  };
};
