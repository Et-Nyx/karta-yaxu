// @ts-nocheck
import { db } from '$lib/db';
import { eq, asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load = async ({ params }: Parameters<PageServerLoad>[0]) => {
  const { slug } = params;

  // 1. Fetch Move
  const mList = await db.select({
    id: schema.moves.id,
    name: schema.moves.name,
    power: schema.moves.power,
    accuracy: schema.moves.accuracy,
    pp: schema.moves.pp,
    damageClass: schema.moves.damageClass,
    priority: schema.moves.priority,
    target: schema.moves.target,
    typeName: schema.types.name,
  })
  .from(schema.moves)
  .leftJoin(schema.types, eq(schema.moves.typeId, schema.types.id))
  .where(eq(schema.moves.slug, slug))
  .limit(1);

  if (mList.length === 0) {
    throw error(404, 'Move not found');
  }

  const move = mList[0];

  // 2. Fetch Pokemon that learn this move (distinct)
  const pokemonLearnersData = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
  })
  .from(schema.pokemonMoves)
  .innerJoin(schema.pokemon, eq(schema.pokemonMoves.pokemonId, schema.pokemon.id))
  .where(eq(schema.pokemonMoves.moveId, move.id))
  .groupBy(schema.pokemon.id)
  .orderBy(asc(schema.pokemon.pokeapiId));

  // 3. Fetch types for these Pokemon
  // Note: For a very common move like Tackle, this might be 500+ Pokemon.
  // We'll fetch all pokemonTypes for the learners and map them in memory.
  const learnerIds = pokemonLearnersData.map(p => p.id);
  
  let typesMap = new Map<number, string[]>();
  
  // SQLite limits in() clauses, so if learnerIds > 1000 we might need chunks,
  // but pokemon count is ~1010 anyway.
  if (learnerIds.length > 0) {
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

  const learners = pokemonLearnersData.map(p => ({
    ...p,
    types: typesMap.get(p.id) || []
  }));

  return {
    move,
    learners
  };
};
