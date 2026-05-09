import { db } from '$lib/db';
import { eq, asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  const { slug } = params;

  // 1. Fetch Type
  const tList = await db.select()
    .from(schema.types)
    .where(eq(schema.types.slug, slug))
    .limit(1);

  if (tList.length === 0) {
    throw error(404, 'Type not found');
  }

  const typeData = tList[0];

  // 2. Fetch Effectiveness (Attacking)
  const attackingList = await db.select({
    defenderName: schema.types.name,
    multiplier: schema.typeEffectiveness.multiplier,
  })
  .from(schema.typeEffectiveness)
  .innerJoin(schema.types, eq(schema.typeEffectiveness.defenderId, schema.types.id))
  .where(eq(schema.typeEffectiveness.attackerId, typeData.id));

  // 3. Fetch Effectiveness (Defending)
  const defendingList = await db.select({
    attackerName: schema.types.name,
    multiplier: schema.typeEffectiveness.multiplier,
  })
  .from(schema.typeEffectiveness)
  .innerJoin(schema.types, eq(schema.typeEffectiveness.attackerId, schema.types.id))
  .where(eq(schema.typeEffectiveness.defenderId, typeData.id));

  // 4. Fetch Pokemon with this type
  const pokemonData = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
  })
  .from(schema.pokemonTypes)
  .innerJoin(schema.pokemon, eq(schema.pokemonTypes.pokemonId, schema.pokemon.id))
  .where(eq(schema.pokemonTypes.typeId, typeData.id))
  .orderBy(asc(schema.pokemon.pokeapiId));

  // Fetch all types for these pokemon
  const pIds = pokemonData.map(p => p.id);
  let typesMap = new Map<number, string[]>();
  
  if (pIds.length > 0) {
    const pTypes = await db.select({
      pokemonId: schema.pokemonTypes.pokemonId,
      typeName: schema.types.name,
    })
    .from(schema.pokemonTypes)
    .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
    .orderBy(asc(schema.pokemonTypes.slot));

    for (const pt of pTypes) {
      if (!typesMap.has(pt.pokemonId)) {
        typesMap.set(pt.pokemonId, []);
      }
      typesMap.get(pt.pokemonId)!.push(pt.typeName);
    }
  }

  const pokemonList = pokemonData.map(p => ({
    ...p,
    types: typesMap.get(p.id) || []
  }));

  return {
    typeData,
    attackingList,
    defendingList,
    pokemonList
  };
};
