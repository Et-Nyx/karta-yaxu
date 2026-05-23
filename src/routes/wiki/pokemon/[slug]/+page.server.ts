import { db } from '$lib/db';
import { eq, and, asc, desc, inArray, lt, gt, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/sqlite-core';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, url }) => {
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

  // Dex navigation
  const dexList = await db.select({ id: schema.pokedexes.id, name: schema.pokedexes.name, slug: schema.pokedexes.slug })
    .from(schema.pokedexes)
    .innerJoin(schema.pokedexEntries, eq(schema.pokedexEntries.pokedexId, schema.pokedexes.id))
    .where(eq(schema.pokedexEntries.speciesId, species.id))
    .orderBy(asc(schema.pokedexes.id));

  let nav: {
    dexList: typeof dexList;
    selectedDex: (typeof dexList)[0];
    currentEntry: number;
    prev: { entryNumber: number; speciesName: string; pokemonSlug: string } | null;
    next: { entryNumber: number; speciesName: string; pokemonSlug: string } | null;
  } | null = null;

  if (dexList.length > 0) {
    const dexParam = url.searchParams.get('dex');
    const selectedDex = dexList.find(d => d.slug === dexParam) ?? dexList[0];

    const [currentEntryRow] = await db.select({ entryNumber: schema.pokedexEntries.entryNumber })
      .from(schema.pokedexEntries)
      .where(and(
        eq(schema.pokedexEntries.pokedexId, selectedDex.id),
        eq(schema.pokedexEntries.speciesId, species.id)
      ))
      .limit(1);

    const currentEntry = currentEntryRow.entryNumber;

    const prevRows = await db.select({
      entryNumber: schema.pokedexEntries.entryNumber,
      speciesName: schema.pokemonSpecies.name,
      pokemonSlug: schema.pokemon.slug,
    })
    .from(schema.pokedexEntries)
    .innerJoin(schema.pokemonSpecies, eq(schema.pokedexEntries.speciesId, schema.pokemonSpecies.id))
    .innerJoin(schema.pokemon, eq(schema.pokemon.id, schema.pokemonSpecies.defaultPokemonId))
    .where(and(
      eq(schema.pokedexEntries.pokedexId, selectedDex.id),
      lt(schema.pokedexEntries.entryNumber, currentEntry)
    ))
    .orderBy(desc(schema.pokedexEntries.entryNumber))
    .limit(1);

    const nextRows = await db.select({
      entryNumber: schema.pokedexEntries.entryNumber,
      speciesName: schema.pokemonSpecies.name,
      pokemonSlug: schema.pokemon.slug,
    })
    .from(schema.pokedexEntries)
    .innerJoin(schema.pokemonSpecies, eq(schema.pokedexEntries.speciesId, schema.pokemonSpecies.id))
    .innerJoin(schema.pokemon, eq(schema.pokemon.id, schema.pokemonSpecies.defaultPokemonId))
    .where(and(
      eq(schema.pokedexEntries.pokedexId, selectedDex.id),
      gt(schema.pokedexEntries.entryNumber, currentEntry)
    ))
    .orderBy(asc(schema.pokedexEntries.entryNumber))
    .limit(1);

    nav = {
      dexList,
      selectedDex,
      currentEntry,
      prev: prevRows[0] ?? null,
      next: nextRows[0] ?? null,
    };
  }

  // 3. Fetch Types (with IDs for type matchup)
  const typesData = await db.select({
    id: schema.types.id,
    name: schema.types.name,
    hexColor: schema.types.hexColor,
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

  const statOrder = ['hp', 'attack', 'defense', 'special-attack', 'special-defense', 'speed'];
  statsData.sort((a, b) => statOrder.indexOf(a.statName) - statOrder.indexOf(b.statName));

  // 6. Version Mechanics (Flavor Texts & Learnsets)
  const availableVersions = await db.select({
    id: schema.versionGroups.id,
    name: schema.versionGroups.name,
    order: schema.versionGroups.order
  })
  .from(schema.versionGroups)
  .orderBy(asc(schema.versionGroups.order));

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
    moveName:       schema.moves.name,
    moveSlug:       schema.moves.slug,
    damageClass:    schema.moves.damageClass,
    typeName:       schema.types.name,
    typeHexColor:   schema.types.hexColor,
    power:          schema.moves.power,
    accuracy:       schema.moves.accuracy,
    pp:             schema.moves.pp,
    level:          schema.pokemonMoves.level,
    methodName:     schema.moveLearnMethods.name,
    versionGroupId: schema.pokemonMoves.versionGroupId
  })
  .from(schema.pokemonMoves)
  .innerJoin(schema.moves, eq(schema.pokemonMoves.moveId, schema.moves.id))
  .innerJoin(schema.types, eq(schema.moves.typeId, schema.types.id))
  .innerJoin(schema.moveLearnMethods, eq(schema.pokemonMoves.learnMethodId, schema.moveLearnMethods.id))
  .where(eq(schema.pokemonMoves.pokemonId, pokemon.id))
  .orderBy(asc(schema.pokemonMoves.level));

  // 7. Genus (English)
  const genusRow = await db.select({ genus: schema.pokemonGenera.genus })
    .from(schema.pokemonGenera)
    .innerJoin(schema.languages, eq(schema.pokemonGenera.languageId, schema.languages.id))
    .where(and(
      eq(schema.pokemonGenera.speciesId, species.id),
      eq(schema.languages.name, 'en')
    ))
    .limit(1);

  const genus = genusRow[0]?.genus ?? null;

  // 8. Egg Groups
  const eggGroupsData = await db.select({ name: schema.eggGroups.name })
    .from(schema.speciesEggGroups)
    .innerJoin(schema.eggGroups, eq(schema.speciesEggGroups.eggGroupId, schema.eggGroups.id))
    .where(eq(schema.speciesEggGroups.speciesId, species.id));

  // 9. Type Matchup (defensive effectiveness)
  const typeIds = typesData.map(t => t.id);

  let typeMatchup: { name: string; multiplier: number; hexColor: string | null }[] = [];

  if (typeIds.length > 0) {
    const attackerAlias = alias(schema.types, 'attacker');

    const effectivenessRows = await db.select({
      attackerName:     attackerAlias.name,
      attackerHexColor: attackerAlias.hexColor,
      defenderId:       schema.typeEffectiveness.defenderId,
      multiplier:       schema.typeEffectiveness.multiplier,
    })
    .from(schema.typeEffectiveness)
    .innerJoin(attackerAlias, eq(schema.typeEffectiveness.attackerId, attackerAlias.id))
    .where(inArray(schema.typeEffectiveness.defenderId, typeIds));

    const matchup: Record<string, number> = {};
    const hexColorMap: Record<string, string | null> = {};
    for (const row of effectivenessRows) {
      matchup[row.attackerName] = (matchup[row.attackerName] ?? 1) * row.multiplier;
      hexColorMap[row.attackerName] = row.attackerHexColor;
    }

    typeMatchup = Object.entries(matchup)
      .filter(([, m]) => m !== 1)
      .map(([name, multiplier]) => ({ name, multiplier, hexColor: hexColorMap[name] ?? null }))
      .sort((a, b) => b.multiplier - a.multiplier);
  }

  // 9b. Offensive matchup (best effectiveness per defending type across all of Pokemon's types)
  let offensiveMatchup: { name: string; multiplier: number; hexColor: string | null }[] = [];

  if (typeIds.length > 0) {
    const defenderAlias = alias(schema.types, 'defender');

    const offRows = await db.select({
      defenderName:     defenderAlias.name,
      defenderHexColor: defenderAlias.hexColor,
      multiplier:       schema.typeEffectiveness.multiplier,
    })
    .from(schema.typeEffectiveness)
    .innerJoin(defenderAlias, eq(schema.typeEffectiveness.defenderId, defenderAlias.id))
    .where(inArray(schema.typeEffectiveness.attackerId, typeIds));

    const offMap: Record<string, number> = {};
    const offHexMap: Record<string, string | null> = {};
    for (const row of offRows) {
      const prev = offMap[row.defenderName];
      offMap[row.defenderName] = prev === undefined ? row.multiplier : Math.max(prev, row.multiplier);
      offHexMap[row.defenderName] = row.defenderHexColor;
    }

    offensiveMatchup = Object.entries(offMap)
      .filter(([, m]) => m !== 1)
      .map(([name, multiplier]) => ({ name, multiplier, hexColor: offHexMap[name] ?? null }))
      .sort((a, b) => b.multiplier - a.multiplier);
  }

  // 10. Evolution Chain (linked-list: walk up to root via pokemon.evolves_from_pokemon_id, then BFS down)
  let evolutionChain: {
    fromSpeciesId: number;
    fromName: string;
    fromSlug: string;
    fromSprite: string | null;
    toSpeciesId: number;
    toName: string;
    toSlug: string;
    toSprite: string | null;
    trigger: string;
    minLevel: number | null;
    minHappiness: number | null;
    itemName: string | null;
    itemSlug: string | null;
    timeOfDay: string | null;
    location: string | null;
  }[] = [];

  {
    // Walk up to find the root pokemon id (iterative; trees are ≤ 4 levels deep in canon)
    // Always start from the default/base Pokemon of the species to let alternate/regional/battle forms
    // inherit the complete evolutionary lineage of the species family!
    let rootId = species.defaultPokemonId ?? pokemon.id;
    {
      const [startPk] = await db.select({ e: schema.pokemon.evolvesFromPokemonId })
        .from(schema.pokemon).where(eq(schema.pokemon.id, rootId)).limit(1);
      const seen = new Set<number>([rootId]);
      let cursor: number | null = startPk?.e ?? null;
      while (cursor != null && !seen.has(cursor)) {
        seen.add(cursor);
        rootId = cursor;
        const [parent] = await db.select({ p: schema.pokemon.evolvesFromPokemonId })
          .from(schema.pokemon).where(eq(schema.pokemon.id, cursor)).limit(1);
        cursor = parent?.p ?? null;
      }
    }

    // BFS down from root to collect all descendants and build edges
    const descendantIds: number[] = [rootId];
    const edges: { fromPokemonId: number; toPokemonId: number }[] = [];
    let frontier = [rootId];
    const seenDown = new Set<number>([rootId]);
    while (frontier.length > 0) {
      const children = await db.select({ id: schema.pokemon.id, parent: schema.pokemon.evolvesFromPokemonId })
        .from(schema.pokemon)
        .where(inArray(schema.pokemon.evolvesFromPokemonId, frontier));
      const nextFrontier: number[] = [];
      for (const c of children) {
        if (seenDown.has(c.id)) continue;
        seenDown.add(c.id);
        descendantIds.push(c.id);
        edges.push({ fromPokemonId: c.parent!, toPokemonId: c.id });
        nextFrontier.push(c.id);
      }
      frontier = nextFrontier;
    }

    if (edges.length > 0) {
      // Resolve each edge's display data via a single batched query
      const childIds = edges.map(e => e.toPokemonId);

      const childPokemon = await db.select({
        id:        schema.pokemon.id,
        speciesId: schema.pokemon.speciesId,
        slug:      schema.pokemon.slug,
        sprite:    schema.pokemon.spriteDefault,
        speciesName: schema.pokemonSpecies.name,
        speciesSlug: schema.pokemonSpecies.slug,
      })
        .from(schema.pokemon)
        .innerJoin(schema.pokemonSpecies, eq(schema.pokemon.speciesId, schema.pokemonSpecies.id))
        .where(inArray(schema.pokemon.id, childIds));

      const parentIds = [...new Set(edges.map(e => e.fromPokemonId))];
      const parentPokemon = await db.select({
        id:        schema.pokemon.id,
        slug:      schema.pokemon.slug,
        sprite:    schema.pokemon.spriteDefault,
        speciesId: schema.pokemon.speciesId,
        speciesName: schema.pokemonSpecies.name,
        speciesSlug: schema.pokemonSpecies.slug,
      })
        .from(schema.pokemon)
        .innerJoin(schema.pokemonSpecies, eq(schema.pokemon.speciesId, schema.pokemonSpecies.id))
        .where(inArray(schema.pokemon.id, parentIds));

      const details = await db.select({
        pokemonId:    schema.pokemonEvolutionDetails.pokemonId,
        trigger:      schema.pokemonEvolutionDetails.trigger,
        minLevel:     schema.pokemonEvolutionDetails.minLevel,
        minHappiness: schema.pokemonEvolutionDetails.minHappiness,
        itemId:       schema.pokemonEvolutionDetails.itemId,
        timeOfDay:    schema.pokemonEvolutionDetails.timeOfDay,
        location:     schema.pokemonEvolutionDetails.location,
      })
        .from(schema.pokemonEvolutionDetails)
        .where(inArray(schema.pokemonEvolutionDetails.pokemonId, childIds));

      const itemIds = details.map(d => d.itemId).filter((v): v is number => v != null);
      const itemRows = itemIds.length > 0
        ? await db.select({ id: schema.items.id, name: schema.items.name, slug: schema.items.slug })
            .from(schema.items).where(inArray(schema.items.id, itemIds))
        : [];

      const childById  = new Map(childPokemon.map(c => [c.id, c]));
      const parentById = new Map(parentPokemon.map(p => [p.id, p]));
      const detailByChild = new Map(details.map(d => [d.pokemonId, d]));
      const itemById = new Map(itemRows.map(i => [i.id, i]));

      for (const e of edges) {
        const c = childById.get(e.toPokemonId);
        const p = parentById.get(e.fromPokemonId);
        const d = detailByChild.get(e.toPokemonId);
        if (!c || !p) continue;
        const item = d?.itemId != null ? itemById.get(d.itemId) : null;
        evolutionChain.push({
          fromSpeciesId: p.speciesId,
          fromName:      p.speciesName,
          fromSlug:      p.speciesSlug,
          fromSprite:    p.sprite ?? null,
          toSpeciesId:   c.speciesId,
          toName:        c.speciesName,
          toSlug:        c.speciesSlug,
          toSprite:      c.sprite ?? null,
          trigger:       d?.trigger ?? 'unknown',
          minLevel:      d?.minLevel ?? null,
          minHappiness:  d?.minHappiness ?? null,
          itemName:      item?.name ?? null,
          itemSlug:      item?.slug ?? null,
          timeOfDay:     d?.timeOfDay ?? null,
          location:      d?.location ?? null,
        });
      }
    }
  }

  // 11. Chain species data — all forms per species (form-aware chain display)
  const CHAIN_SKIP_PATTERNS = ['-totem', '-cap', '-cosplay', '-rock-star', '-belle', '-pop-star', '-phd', '-libre', '-starter'];
  const REGIONAL_SUFFIXES = ['alola', 'galar', 'hisui', 'paldea'];

  type ChainForm = {
    pokeapiId: number | null;
    name: string;
    slug: string;
    spriteUrl: string | null;
    isDefault: boolean;
    types: Array<{ name: string; hexColor: string | null }>;
    dexNumber: number;
  };

  type ChainSpecies = {
    speciesId: number;
    forms: ChainForm[];
  };

  let chainSpecies: ChainSpecies[] = [];

  {
    const chainSpeciesIds = [...new Set([
      species.id,
      ...evolutionChain.map(l => l.fromSpeciesId),
      ...evolutionChain.map(l => l.toSpeciesId),
    ])];

    const chainPokemonAlias = alias(schema.pokemon, 'chain_pokemon');

    const chainRows = await db.select({
      speciesId:    schema.pokemonSpecies.id,
      dexNumber:    schema.pokemonSpecies.dexNumber,
      pokeapiId:    chainPokemonAlias.pokeapiId,
      name:         chainPokemonAlias.name,
      slug:         chainPokemonAlias.slug,
      spriteDefault: chainPokemonAlias.spriteDefault,
      spriteArtwork: chainPokemonAlias.spriteArtwork,
      isDefault:    sql<boolean>`(${chainPokemonAlias.id} = ${schema.pokemonSpecies.defaultPokemonId})`,
      typeName:     schema.types.name,
      typeHexColor: schema.types.hexColor,
    })
    .from(schema.pokemonSpecies)
    .innerJoin(chainPokemonAlias, eq(chainPokemonAlias.speciesId, schema.pokemonSpecies.id))
    .innerJoin(schema.pokemonTypes, eq(schema.pokemonTypes.pokemonId, chainPokemonAlias.id))
    .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
    .where(inArray(schema.pokemonSpecies.id, chainSpeciesIds))
    .orderBy(desc(sql<boolean>`(${chainPokemonAlias.id} = ${schema.pokemonSpecies.defaultPokemonId})`), asc(chainPokemonAlias.name), asc(schema.pokemonTypes.slot));

    // Accumulate: speciesId → forms (deduplicated by slug, types accumulated)
    const speciesMap = new Map<number, { speciesId: number; formMap: Map<string, ChainForm> }>();
    for (const row of chainRows) {
      if (CHAIN_SKIP_PATTERNS.some(p => row.slug.includes(p))) continue;

      if (!speciesMap.has(row.speciesId)) {
        speciesMap.set(row.speciesId, { speciesId: row.speciesId, formMap: new Map() });
      }
      const entry = speciesMap.get(row.speciesId)!;
      if (!entry.formMap.has(row.slug)) {
        entry.formMap.set(row.slug, {
          pokeapiId: row.pokeapiId,
          name: row.name,
          slug: row.slug,
          spriteUrl: row.spriteDefault ?? row.spriteArtwork,
          isDefault: row.isDefault,
          dexNumber: row.dexNumber,
          types: [],
        });
      }
      entry.formMap.get(row.slug)!.types.push({ name: row.typeName, hexColor: row.typeHexColor });
    }

    chainSpecies = [...speciesMap.values()].map(({ speciesId, formMap }) => ({
      speciesId,
      forms: [...formMap.values()],
    }));
  }

  // Which regional suffix the current Pokémon belongs to (e.g. "galar" for mr-mime-galar)
  const formSuffix = REGIONAL_SUFFIXES.find(r =>
    pokemon.slug.endsWith('-' + r) || pokemon.slug.includes('-' + r + '-')
  ) ?? null;

  // PokeAPI does not encode which regional form is required for cross-gen Galarian evolutions.
  // These are known cases where only a specific regional form can produce the evolution.
  const FORM_RESTRICTED_EVOLUTIONS = [
    { fromSlug: 'mr-mime',   toSlug: 'mr-rime',    requiredSuffix: 'galar' },
    { fromSlug: 'meowth',    toSlug: 'perrserker', requiredSuffix: 'galar' },
    { fromSlug: 'farfetchd', toSlug: 'sirfetchd',  requiredSuffix: 'galar' },
    { fromSlug: 'corsola',   toSlug: 'cursola',    requiredSuffix: 'galar' },
    { fromSlug: 'yamask',    toSlug: 'runerigus',  requiredSuffix: 'galar' },
    { fromSlug: 'linoone',   toSlug: 'obstagoon',  requiredSuffix: 'galar' },
  ];

  const evolutionChainFinal = evolutionChain.map(edge => ({
    ...edge,
    requiredSuffix: FORM_RESTRICTED_EVOLUTIONS.find(
      r => edge.fromSlug === r.fromSlug && edge.toSlug === r.toSlug
    )?.requiredSuffix ?? null,
  }));

  // 12. Location encounters (wild encounter data)
  const locationEncounters = await db.selectDistinct({
    locationName:   schema.locations.name,
    locationSlug:   schema.locations.slug,
    areaName:       schema.locationAreas.name,
    methodName:     schema.encounterMethods.name,
    methodOrder:    schema.encounterMethods.order,
    minLevel:       schema.encounters.minLevel,
    maxLevel:       schema.encounters.maxLevel,
    chance:         schema.encounters.chance,
    versionGroupId: schema.versionGroups.id,
  })
  .from(schema.encounters)
  .innerJoin(schema.locationAreas, eq(schema.encounters.locationAreaId, schema.locationAreas.id))
  .innerJoin(schema.locations, eq(schema.locationAreas.locationId, schema.locations.id))
  .innerJoin(schema.encounterMethods, eq(schema.encounters.encounterMethodId, schema.encounterMethods.id))
  .innerJoin(schema.versions, eq(schema.encounters.versionId, schema.versions.id))
  .innerJoin(schema.versionGroups, eq(schema.versions.versionGroupId, schema.versionGroups.id))
  .where(eq(schema.encounters.pokemonId, pokemon.id))
  .orderBy(
    asc(schema.locations.name),
    asc(schema.locationAreas.gameIndex),
    asc(schema.encounterMethods.order),
    desc(schema.encounters.chance),
  );

  return {
    pokemon,
    species,
    types: typesData.map(({ id: _id, slot: _slot, ...rest }) => rest),
    abilities: abilitiesData,
    stats: statsData,
    availableVersions,
    flavorTexts,
    learnset,
    locationEncounters,
    genus,
    eggGroups: eggGroupsData,
    typeMatchup,
    offensiveMatchup,
    evolutionChain: evolutionChainFinal,
    chainSpecies,
    formSuffix,
    nav,
  };
};
