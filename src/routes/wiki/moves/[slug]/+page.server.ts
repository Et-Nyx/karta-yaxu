import { db } from '$lib/db';
import { eq, asc, and } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
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
    typeHexColor: schema.types.hexColor,
  })
  .from(schema.moves)
  .leftJoin(schema.types, eq(schema.moves.typeId, schema.types.id))
  .where(eq(schema.moves.slug, slug))
  .limit(1);

  if (mList.length === 0) {
    throw error(404, 'Move not found');
  }

  const move = mList[0];

  // 2. Fetch English language ID
  const englishLang = await db.select({ id: schema.languages.id })
    .from(schema.languages)
    .where(eq(schema.languages.name, 'en'))
    .limit(1);
  const englishId = englishLang[0]?.id ?? null;

  // 3. Fetch effect text
  const effectRows = englishId ? await db.select({
    shortEffect: schema.moveEffects.shortEffect,
    effect: schema.moveEffects.effect,
  })
  .from(schema.moveEffects)
  .where(and(
    eq(schema.moveEffects.moveId, move.id),
    eq(schema.moveEffects.languageId, englishId)
  ))
  .limit(1) : [];

  // 4. Fetch flavor texts per version group
  const flavorTexts = englishId ? await db.select({
    flavorText: schema.moveFlavorTexts.flavorText,
    versionGroupId: schema.moveFlavorTexts.versionGroupId,
    versionGroupName: schema.versionGroups.name,
  })
  .from(schema.moveFlavorTexts)
  .innerJoin(schema.versionGroups, eq(schema.moveFlavorTexts.versionGroupId, schema.versionGroups.id))
  .where(and(
    eq(schema.moveFlavorTexts.moveId, move.id),
    eq(schema.moveFlavorTexts.languageId, englishId)
  ))
  .orderBy(asc(schema.versionGroups.order)) : [];

  // 5. Fetch Pokemon that learn this move (distinct)
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
  
  let typesMap = new Map<number, Array<{ name: string; hexColor: string | null }>>();

  // SQLite limits in() clauses, so if learnerIds > 1000 we might need chunks,
  // but pokemon count is ~1010 anyway.
  if (learnerIds.length > 0) {
    const pTypes = await db.select({
      pokemonId: schema.pokemonTypes.pokemonId,
      typeName: schema.types.name,
      typeHexColor: schema.types.hexColor,
    })
    .from(schema.pokemonTypes)
    .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
    .orderBy(asc(schema.pokemonTypes.slot));

    for (const pt of pTypes) {
      if (!typesMap.has(pt.pokemonId)) {
        typesMap.set(pt.pokemonId, []);
      }
      typesMap.get(pt.pokemonId)!.push({ name: pt.typeName, hexColor: pt.typeHexColor });
    }
  }

  const learners = pokemonLearnersData.map(p => ({
    ...p,
    types: typesMap.get(p.id) || []
  }));

  return {
    move,
    learners,
    shortEffect: effectRows[0]?.shortEffect ?? null,
    effect: effectRows[0]?.effect ?? null,
    flavorTexts,
  };
};
