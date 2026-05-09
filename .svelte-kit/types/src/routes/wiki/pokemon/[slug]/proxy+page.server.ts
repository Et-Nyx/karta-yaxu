// @ts-nocheck
import { db } from '$lib/db';
import { eq, and, asc, inArray } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load = async ({ params }: Parameters<PageServerLoad>[0]) => {
  const { slug } = params;

  // 1. Fetch Pokemon
  const pList = await db.select()
    .from(schema.pokemon)
    .where(eq(schema.pokemon.slug, slug))
    .limit(1);

  if (pList.length === 0) {
    throw error(404, 'Pokémon not found');
  }

  const pokemon = pList[0];

  // 2. Fetch Species
  const speciesList = await db.select()
    .from(schema.pokemonSpecies)
    .where(eq(schema.pokemonSpecies.id, pokemon.speciesId))
    .limit(1);
    
  const species = speciesList[0];

  // 3. Fetch Types
  const typesData = await db.select({
    name: schema.types.name,
    slot: schema.pokemonTypes.slot,
  })
  .from(schema.pokemonTypes)
  .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
  .where(eq(schema.pokemonTypes.pokemonId, pokemon.id))
  .orderBy(asc(schema.pokemonTypes.slot));

  // 4. Fetch Abilities
  const abilitiesData = await db.select({
    name: schema.abilities.name,
    slug: schema.abilities.slug,
    isHidden: schema.pokemonAbilities.isHidden,
    slot: schema.pokemonAbilities.slot,
  })
  .from(schema.pokemonAbilities)
  .innerJoin(schema.abilities, eq(schema.pokemonAbilities.abilityId, schema.abilities.id))
  .where(eq(schema.pokemonAbilities.pokemonId, pokemon.id))
  .orderBy(asc(schema.pokemonAbilities.slot));

  // 5. Fetch Stats
  const statsData = await db.select({
    statName: schema.pokemonStats.statName,
    baseStat: schema.pokemonStats.baseStat,
    effort: schema.pokemonStats.effort,
  })
  .from(schema.pokemonStats)
  .where(eq(schema.pokemonStats.pokemonId, pokemon.id));

  // Sort stats in traditional order
  const statOrder = ['hp', 'attack', 'defense', 'special-attack', 'special-defense', 'speed'];
  statsData.sort((a, b) => statOrder.indexOf(a.statName) - statOrder.indexOf(b.statName));

  // 6. Fetch Evolution Chain
  let evolutionNodes = [];
  let evolutionEdges = [];

  if (species.evolutionChainId) {
    evolutionEdges = await db.select()
      .from(schema.evolutionDetails)
      .where(eq(schema.evolutionDetails.chainId, species.evolutionChainId));

    evolutionNodes = await db.select({
      speciesId: schema.pokemonSpecies.id,
      pokeapiId: schema.pokemon.pokeapiId,
      name: schema.pokemon.name,
      slug: schema.pokemon.slug,
      spriteDefault: schema.pokemon.spriteDefault,
      isBaby: schema.pokemonSpecies.isBaby,
      order: schema.pokemonSpecies.order
    })
    .from(schema.pokemonSpecies)
    .innerJoin(schema.pokemon, eq(schema.pokemonSpecies.id, schema.pokemon.speciesId))
    .where(
      and(
        eq(schema.pokemonSpecies.evolutionChainId, species.evolutionChainId),
        eq(schema.pokemon.isDefault, true)
      )
    );

    // Buscar TYPES de cada Pokémon da cadeia evolutiva
    if (evolutionNodes.length > 0) {
      const nodeIds = evolutionNodes.map(n => n.speciesId).filter(Boolean);

      const evoTypes = await db.select({
        speciesId: schema.pokemonSpecies.id,
        typeName: schema.types.name,
      })
      .from(schema.pokemonTypes)
      .innerJoin(schema.pokemon, eq(schema.pokemonTypes.pokemonId, schema.pokemon.id))
      .innerJoin(schema.pokemonSpecies, eq(schema.pokemon.speciesId, schema.pokemonSpecies.id))
      .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
      .where(inArray(schema.pokemonSpecies.id, nodeIds))
      .orderBy(asc(schema.pokemonTypes.slot));

      // Agrupar types por species
      const typesMap = new Map<number, string[]>();
      for (const et of evoTypes) {
        if (!typesMap.has(et.speciesId)) {
          typesMap.set(et.speciesId, []);
        }
        typesMap.get(et.speciesId)!.push(et.typeName);
      }

      // Anexar types aos nodes
      evolutionNodes = evolutionNodes.map(n => ({
        ...n,
        types: typesMap.get(n.speciesId) || []
      }));
    }
  }

  // 7. Fetch Version Mechanics (Flavor Texts & Learnsets)
  // Fetch all version groups the pokemon has data for
  const availableVersions = await db.select({
    id: schema.versionGroups.id,
    name: schema.versionGroups.name,
    order: schema.versionGroups.order
  })
  .from(schema.versionGroups)
  .orderBy(asc(schema.versionGroups.order)); // Will filter down to used ones in UI if needed

  const flavorTexts = await db.select({
    flavorText: schema.pokemonFlavorTexts.flavorText,
    versionGroupId: schema.pokemonFlavorTexts.versionGroupId,
    languageId: schema.pokemonFlavorTexts.languageId
  })
  .from(schema.pokemonFlavorTexts)
  .innerJoin(schema.languages, eq(schema.pokemonFlavorTexts.languageId, schema.languages.id))
  .where(
    and(
      eq(schema.pokemonFlavorTexts.speciesId, species.id),
      eq(schema.languages.name, 'en')
    )
  );

  const learnset = await db.select({
    moveName: schema.moves.name,
    moveSlug: schema.moves.slug,
    damageClass: schema.moves.damageClass,
    typeName: schema.types.name,
    level: schema.pokemonMoves.level,
    methodName: schema.moveLearnMethods.name,
    versionGroupId: schema.pokemonMoves.versionGroupId
  })
  .from(schema.pokemonMoves)
  .innerJoin(schema.moves, eq(schema.pokemonMoves.moveId, schema.moves.id))
  .innerJoin(schema.types, eq(schema.moves.typeId, schema.types.id))
  .innerJoin(schema.moveLearnMethods, eq(schema.pokemonMoves.learnMethodId, schema.moveLearnMethods.id))
  .where(eq(schema.pokemonMoves.pokemonId, pokemon.id))
  .orderBy(asc(schema.pokemonMoves.level));

  return {
    pokemon,
    species,
    types: typesData,
    abilities: abilitiesData,
    stats: statsData,
    evolutionNodes,
    evolutionEdges,
    availableVersions,
    flavorTexts,
    learnset
  };
};
