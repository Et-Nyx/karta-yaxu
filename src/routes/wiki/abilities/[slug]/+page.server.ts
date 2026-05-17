import { db } from '$lib/db';
import { eq, asc, and } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  const { slug } = params;

  // 1. Fetch Ability
  const aList = await db.select()
    .from(schema.abilities)
    .where(eq(schema.abilities.slug, slug))
    .limit(1);

  if (aList.length === 0) {
    throw error(404, 'Ability not found');
  }

  const ability = aList[0];

  // 2. Fetch English effect + flavor texts
  const englishLang = await db.select({ id: schema.languages.id })
    .from(schema.languages)
    .where(eq(schema.languages.name, 'en'))
    .limit(1);
  const englishId = englishLang[0]?.id;

  const effectRow = englishId ? await db.select({
    shortEffect: schema.abilityEffects.shortEffect,
    effect: schema.abilityEffects.effect,
  })
  .from(schema.abilityEffects)
  .where(and(
    eq(schema.abilityEffects.abilityId, ability.id),
    eq(schema.abilityEffects.languageId, englishId)
  ))
  .limit(1) : [];

  const flavorTexts = englishId ? await db.select({
    flavorText: schema.abilityFlavorTexts.flavorText,
    versionGroupId: schema.abilityFlavorTexts.versionGroupId,
    versionGroupName: schema.versionGroups.name,
  })
  .from(schema.abilityFlavorTexts)
  .innerJoin(schema.versionGroups, eq(schema.abilityFlavorTexts.versionGroupId, schema.versionGroups.id))
  .where(and(
    eq(schema.abilityFlavorTexts.abilityId, ability.id),
    eq(schema.abilityFlavorTexts.languageId, englishId)
  ))
  .orderBy(asc(schema.versionGroups.order)) : [];

  // 3. Fetch Pokemon with this ability
  const pokemonData = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
    isHidden: schema.pokemonAbilities.isHidden,
  })
  .from(schema.pokemonAbilities)
  .innerJoin(schema.pokemon, eq(schema.pokemonAbilities.pokemonId, schema.pokemon.id))
  .where(eq(schema.pokemonAbilities.abilityId, ability.id))
  .orderBy(asc(schema.pokemon.pokeapiId));

  // 3. Fetch types for these Pokemon
  const pIds = pokemonData.map(p => p.id);
  
  let typesMap = new Map<number, Array<{ name: string; hexColor: string | null }>>();

  if (pIds.length > 0) {
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

  const pokemonList = pokemonData.map(p => ({
    ...p,
    types: typesMap.get(p.id) || []
  }));

  return {
    ability,
    pokemonList,
    shortEffect: effectRow[0]?.shortEffect ?? null,
    effect: effectRow[0]?.effect ?? null,
    flavorTexts,
  };
};
