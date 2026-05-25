import { error, fail, redirect } from '@sveltejs/kit';
import { z } from 'zod';
import fs from 'fs';
import path from 'path';
import { db } from '$lib/db';
import { pokemon, pokemonAbilities, pokemonEvolutionDetails, pokemonForms, pokemonGenera, pokemonMoves, pokemonSpecies, pokemonTypes, eggGroups, speciesEggGroups, pokedexEntries, pokemonFlavorTexts } from '$lib/db/schema/pokemon';
import { abilities, moveLearnMethods, moves, types, typeEffectiveness } from '$lib/db/schema/core-data';
import { languages, versionGroups, pokedexes } from '$lib/db/schema/game-metadata';
import { items } from '$lib/db/schema/items';
import { and, asc, count, desc, eq, inArray, sql, lt, gt } from 'drizzle-orm';
import { alias } from 'drizzle-orm/sqlite-core';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const intOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
  z.number().int().nullable(),
);

const stat = z.coerce.number().int().min(0).max(255);

const UpdateSchema = z.object({
  name:          z.string().trim().min(1).max(80),
  slug:          z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  speciesId:     z.coerce.number().int().positive(),
  height:        z.coerce.number().int().min(0),
  weight:        z.coerce.number().int().min(0),
  hp:            stat,
  atk:           stat,
  def:           stat,
  spa:           stat,
  spd:           stat,
  spe:           stat,
  spriteDefault: z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteShiny:   z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteArtwork: z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteArtworkShiny: z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteBackDefault:  z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteBackShiny:    z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  slot1TypeId:    z.coerce.number().int().positive(),
  slot2TypeId:    z.preprocess((v) => (v === '' || v == null ? null : Number(v)), z.number().int().positive().nullable()),
  slot1AbilityId: z.coerce.number().int().positive(),
  slot2AbilityId: z.preprocess((v) => (v === '' || v == null ? null : Number(v)), z.number().int().positive().nullable()),
  hiddenAbilityId: z.preprocess((v) => (v === '' || v == null ? null : Number(v)), z.number().int().positive().nullable()),
});

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const textOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : String(v)),
  z.string().nullable(),
);

const EVO_TRIGGERS = ['level-up', 'use-item', 'use-move', 'trade', 'three-critical-hits', 'shed', 'spin', 'tower-of-darkness', 'tower-of-waters', 'agile-style-move', 'strong-style-move'] as const;

const SetPredecessorSchema = z.object({
  parentPokemonId: z.coerce.number().int().positive(),
  trigger:         z.enum(EVO_TRIGGERS),
  minLevel:        intOrNull,
  itemId:          intOrNull,
  heldItemId:      intOrNull,
  knownMoveId:     intOrNull,
  knownMoveTypeId: intOrNull,
  minHappiness:    intOrNull,
  timeOfDay:       textOrNull,
  gender:          intOrNull,
  location:        textOrNull,
});

const UpdateConditionsSchema = SetPredecessorSchema.omit({ parentPokemonId: true });

const LearnsetEntrySchema = z.object({
  moveId:         z.coerce.number().int().positive(),
  learnMethodId:  z.coerce.number().int().positive(),
  level:          z.coerce.number().int().min(0).max(100).default(0),
  versionGroupId: z.coerce.number().int().positive(),
});

const LearnsetUpdateSchema = z.object({
  oldMoveId:         z.coerce.number().int().positive(),
  oldLearnMethodId:  z.coerce.number().int().positive(),
  oldLevel:          z.coerce.number().int().min(0),
  oldVersionGroupId: z.coerce.number().int().positive(),
  newMoveId:         z.coerce.number().int().positive(),
  newLearnMethodId:  z.coerce.number().int().positive(),
  newLevel:          z.coerce.number().int().min(0).max(100),
  newVersionGroupId: z.coerce.number().int().positive(),
});

export const load: PageServerLoad = async ({ params, url }) => {
  const [row] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Pokémon not found');

  const typeRows = await db.select({
    slot:     pokemonTypes.slot,
    typeId:   pokemonTypes.typeId,
    typeName: types.name,
    typeSlug: types.slug,
    hexColor: types.hexColor,
  })
    .from(pokemonTypes)
    .innerJoin(types, eq(pokemonTypes.typeId, types.id))
    .where(eq(pokemonTypes.pokemonId, row.id))
    .orderBy(asc(pokemonTypes.slot));

  const abilityRows = await db.select({
    abilityId:   pokemonAbilities.abilityId,
    slot:        pokemonAbilities.slot,
    isHidden:    pokemonAbilities.isHidden,
    abilityName: abilities.name,
    abilitySlug: abilities.slug,
  })
    .from(pokemonAbilities)
    .innerJoin(abilities, eq(pokemonAbilities.abilityId, abilities.id))
    .where(eq(pokemonAbilities.pokemonId, row.id))
    .orderBy(asc(pokemonAbilities.slot));

  const speciesList = await db.select({
    id:        pokemonSpecies.id,
    name:      pokemonSpecies.name,
    slug:      pokemonSpecies.slug,
    dexNumber: pokemonSpecies.dexNumber,
  }).from(pokemonSpecies).orderBy(asc(pokemonSpecies.dexNumber));

  const typeList = await db.select({
    id:       types.id,
    name:     types.name,
    slug:     types.slug,
    hexColor: types.hexColor,
  }).from(types).orderBy(asc(types.id));

  const abilityList = await db.select({
    id:   abilities.id,
    name: abilities.name,
    slug: abilities.slug,
  }).from(abilities).orderBy(asc(abilities.name));

  // All pokemon of the same species (for form sub-tabs: ALOLA, MEGA, GMAX, etc.)
  const FORM_SKIP = ['-totem', '-cap', '-cosplay', '-rock-star', '-belle', '-pop-star', '-phd', '-libre', '-starter'];
  const speciesFormRows = await db.select({
    pokeapiId:    pokemon.pokeapiId,
    name:         pokemon.name,
    slug:         pokemon.slug,
    isDefault:    sql<boolean>`(${pokemon.id} = ${pokemonSpecies.defaultPokemonId})`,
    spriteDefault: pokemon.spriteDefault,
    spriteArtwork: pokemon.spriteArtwork,
    typeName:     types.name,
    typeHexColor: types.hexColor,
  })
    .from(pokemon)
    .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
    .innerJoin(pokemonTypes, eq(pokemonTypes.pokemonId, pokemon.id))
    .innerJoin(types, eq(pokemonTypes.typeId, types.id))
    .where(eq(pokemon.speciesId, row.speciesId))
    .orderBy(desc(sql<boolean>`(${pokemon.id} = ${pokemonSpecies.defaultPokemonId})`), asc(pokemon.name), asc(pokemonTypes.slot));

  const formMap = new Map<string, {
    pokeapiId: number | null; name: string; slug: string;
    isDefault: boolean; spriteUrl: string | null;
    types: { name: string; hexColor: string | null }[];
  }>();
  for (const r of speciesFormRows) {
    if (FORM_SKIP.some(p => r.slug.includes(p))) continue;
    if (!formMap.has(r.slug)) {
      formMap.set(r.slug, {
        pokeapiId: r.pokeapiId, name: r.name, slug: r.slug,
        isDefault: r.isDefault, spriteUrl: r.spriteDefault ?? r.spriteArtwork,
        types: [],
      });
    }
    formMap.get(r.slug)!.types.push({ name: r.typeName, hexColor: r.typeHexColor });
  }
  const speciesForms = [...formMap.values()];

  // Evolution data: predecessor + its conditions, and list of successors
  let predecessor: { id: number; slug: string; name: string } | null = null;
  let predecessorConditions: typeof pokemonEvolutionDetails.$inferSelect | null = null;
  if (row.evolvesFromPokemonId) {
    const [parent] = await db.select({ id: pokemon.id, slug: pokemon.slug, name: pokemon.name })
      .from(pokemon).where(eq(pokemon.id, row.evolvesFromPokemonId)).limit(1);
    if (parent) predecessor = parent;
    const [cond] = await db.select().from(pokemonEvolutionDetails)
      .where(eq(pokemonEvolutionDetails.pokemonId, row.id)).limit(1);
    if (cond) predecessorConditions = cond;
  }

  const successorRows = await db.select({
    id:       pokemon.id,
    slug:     pokemon.slug,
    name:     pokemon.name,
    trigger:  pokemonEvolutionDetails.trigger,
    conditions: pokemonEvolutionDetails.conditions,
  })
    .from(pokemon)
    .leftJoin(pokemonEvolutionDetails, eq(pokemonEvolutionDetails.pokemonId, pokemon.id))
    .where(eq(pokemon.evolvesFromPokemonId, row.id))
    .orderBy(asc(pokemon.id));

  // Full evolution chain: walk up to root, BFS down, batch-load species node data
  let evolutionChain: {
    fromPokemonId: number;
    fromSpeciesId: number;
    fromName: string;
    fromSlug: string;
    fromSprite: string | null;
    toPokemonId: number;
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
    requiredSuffix: string | null;
  }[] = [];

  {
    // Find all pokemon variants for this species
    const speciesPokemon = await db.select({ id: pokemon.id, evolvesFromPokemonId: pokemon.evolvesFromPokemonId })
      .from(pokemon)
      .where(eq(pokemon.speciesId, row.speciesId));

    // For each variant, walk up to its root
    const roots = new Set<number>();
    for (const pk of speciesPokemon) {
      let rootId = pk.id;
      const seen = new Set<number>([rootId]);
      let cursor = pk.evolvesFromPokemonId;
      while (cursor != null && !seen.has(cursor)) {
        seen.add(cursor);
        rootId = cursor;
        const [parent] = await db.select({ p: pokemon.evolvesFromPokemonId })
          .from(pokemon).where(eq(pokemon.id, cursor)).limit(1);
        cursor = parent?.p ?? null;
      }
      roots.add(rootId);
    }

    // BFS down from all roots to collect descendants and build edges
    const edges: { fromPokemonId: number; toPokemonId: number }[] = [];
    const descendantIds = new Set<number>();
    
    for (const rootId of roots) {
      let frontier = [rootId];
      const seenDown = new Set<number>([rootId]);
      descendantIds.add(rootId);
      while (frontier.length > 0) {
        const children = await db.select({ id: pokemon.id, parent: pokemon.evolvesFromPokemonId })
          .from(pokemon)
          .where(inArray(pokemon.evolvesFromPokemonId, frontier));
        const nextFrontier: number[] = [];
        for (const c of children) {
          if (seenDown.has(c.id)) continue;
          seenDown.add(c.id);
          descendantIds.add(c.id);
          edges.push({ fromPokemonId: c.parent!, toPokemonId: c.id });
          nextFrontier.push(c.id);
        }
        frontier = nextFrontier;
      }
    }

    if (edges.length > 0) {
      const childIds = edges.map(e => e.toPokemonId);
      const childPokemon = await db.select({
        id:        pokemon.id,
        speciesId: pokemon.speciesId,
        slug:      pokemon.slug,
        name:      pokemon.name,
        sprite:    pokemon.spriteDefault,
        speciesName: pokemonSpecies.name,
        speciesSlug: pokemonSpecies.slug,
      })
        .from(pokemon)
        .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
        .where(inArray(pokemon.id, childIds));

      const parentIds = [...new Set(edges.map(e => e.fromPokemonId))];
      const parentPokemon = await db.select({
        id:        pokemon.id,
        slug:      pokemon.slug,
        name:      pokemon.name,
        sprite:    pokemon.spriteDefault,
        speciesId: pokemon.speciesId,
        speciesName: pokemonSpecies.name,
        speciesSlug: pokemonSpecies.slug,
      })
        .from(pokemon)
        .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
        .where(inArray(pokemon.id, parentIds));

      const details = await db.select({
        pokemonId:    pokemonEvolutionDetails.pokemonId,
        trigger:      pokemonEvolutionDetails.trigger,
        conditions:   pokemonEvolutionDetails.conditions,
      })
        .from(pokemonEvolutionDetails)
        .where(inArray(pokemonEvolutionDetails.pokemonId, childIds));

      const itemIds = details.map(d => d.conditions?.itemId).filter((v): v is number => v != null);
      const itemRows = itemIds.length > 0
        ? await db.select({ id: items.id, name: items.name, slug: items.slug })
            .from(items).where(inArray(items.id, itemIds))
        : [];

      const childById  = new Map(childPokemon.map(c => [c.id, c]));
      const parentById = new Map(parentPokemon.map(p => [p.id, p]));
      const detailByChild = new Map(details.map(d => [d.pokemonId, d]));
      const itemById = new Map(itemRows.map(i => [i.id, i]));

      const FORM_RESTRICTED_EVOLUTIONS = [
        { fromSlug: 'mr-mime',   toSlug: 'mr-rime',    requiredSuffix: 'galar' },
        { fromSlug: 'meowth',    toSlug: 'perrserker', requiredSuffix: 'galar' },
        { fromSlug: 'farfetchd', toSlug: 'sirfetchd',  requiredSuffix: 'galar' },
        { fromSlug: 'corsola',   toSlug: 'cursola',    requiredSuffix: 'galar' },
        { fromSlug: 'yamask',    toSlug: 'runerigus',  requiredSuffix: 'galar' },
        { fromSlug: 'linoone',   toSlug: 'obstagoon',  requiredSuffix: 'galar' },
      ];

      for (const e of edges) {
        const c = childById.get(e.toPokemonId);
        const p = parentById.get(e.fromPokemonId);
        const d = detailByChild.get(e.toPokemonId);
        if (!c || !p) continue;
        const cond = d?.conditions ?? {};
        const item = cond.itemId != null ? itemById.get(cond.itemId) : null;
        const restricted = FORM_RESTRICTED_EVOLUTIONS.find(
          r => p.speciesSlug === r.fromSlug && c.speciesSlug === r.toSlug
        );
        evolutionChain.push({
          fromPokemonId: p.id,
          fromSpeciesId: p.speciesId,
          fromName:      p.name,
          fromSlug:      p.slug,
          fromSprite:    p.sprite ?? null,
          toPokemonId:   c.id,
          toSpeciesId:   c.speciesId,
          toName:        c.name,
          toSlug:        c.slug,
          toSprite:      c.sprite ?? null,
          trigger:       d?.trigger ?? 'unknown',
          minLevel:      cond.minLevel ?? null,
          minHappiness:  cond.minHappiness ?? null,
          itemName:      item?.name ?? null,
          itemSlug:      item?.slug ?? null,
          timeOfDay:     cond.timeOfDay ?? null,
          location:      cond.location ?? null,
          requiredSuffix: restricted?.requiredSuffix ?? cond.location ?? null,
        });
      }
    }
  }

  // Chain species data: all forms per species (form-aware chain display)
  const CHAIN_SKIP_PATTERNS = ['-totem', '-cap', '-cosplay', '-rock-star', '-belle', '-pop-star', '-phd', '-libre', '-starter'];

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
      row.speciesId,
      ...evolutionChain.map(l => l.fromSpeciesId),
      ...evolutionChain.map(l => l.toSpeciesId),
    ])];

    const chainPokemonAlias = alias(pokemon, 'chain_pokemon');
    const chainForms = await db.select({
      id:            chainPokemonAlias.id,
      pokeapiId:     chainPokemonAlias.pokeapiId,
      slug:          chainPokemonAlias.slug,
      name:          chainPokemonAlias.name,
      spriteArtwork: chainPokemonAlias.spriteArtwork,
      spriteDefault: chainPokemonAlias.spriteDefault,
      speciesId:     chainPokemonAlias.speciesId,
      typeName:      types.name,
      typeHexColor:  types.hexColor,
      isDefault:     sql<boolean>`${chainPokemonAlias.id} = ${pokemonSpecies.defaultPokemonId}`,
      dexNumber:     pokemonSpecies.dexNumber,
    })
      .from(chainPokemonAlias)
      .innerJoin(pokemonSpecies, eq(chainPokemonAlias.speciesId, pokemonSpecies.id))
      .innerJoin(pokemonTypes, eq(pokemonTypes.pokemonId, chainPokemonAlias.id))
      .innerJoin(types, eq(pokemonTypes.typeId, types.id))
      .where(inArray(chainPokemonAlias.speciesId, chainSpeciesIds))
      .orderBy(asc(pokemonSpecies.dexNumber), asc(pokemonTypes.slot));

    const spMap = new Map<number, ChainForm[]>();
    for (const f of chainForms) {
      if (CHAIN_SKIP_PATTERNS.some(pat => f.slug.endsWith(pat))) continue;
      if (!spMap.has(f.speciesId)) spMap.set(f.speciesId, []);
      const forms = spMap.get(f.speciesId)!;
      let existing = forms.find(x => x.slug === f.slug);
      if (!existing) {
        existing = {
          pokeapiId: f.pokeapiId,
          name:      f.name,
          slug:      f.slug,
          spriteUrl: f.spriteDefault ?? f.spriteArtwork,
          isDefault: f.isDefault,
          types:     [],
          dexNumber: f.dexNumber,
        };
        forms.push(existing);
      }
      existing.types.push({ name: f.typeName, hexColor: f.typeHexColor });
    }

    chainSpecies = [...spMap.entries()].map(([speciesId, forms]) => ({ speciesId, forms }));
  }

  // FK dropdowns
  const allPokemon = await db.select({ id: pokemon.id, slug: pokemon.slug, name: pokemon.name, speciesId: pokemon.speciesId })
    .from(pokemon).orderBy(asc(pokemon.name));
  const allItems = await db.select({ id: items.id, name: items.name, slug: items.slug })
    .from(items).orderBy(asc(items.name));
  const allMoves = await db.select({ id: moves.id, name: moves.name, slug: moves.slug, typeId: moves.typeId })
    .from(moves).orderBy(asc(moves.name));

  // Learnset support data
  const vgCounts = await db.select({
    id:    versionGroups.id,
    slug:  versionGroups.slug,
    name:  versionGroups.name,
    count: count(pokemonMoves.moveId),
  })
    .from(versionGroups)
    .leftJoin(pokemonMoves, and(
      eq(pokemonMoves.versionGroupId, versionGroups.id),
      eq(pokemonMoves.pokemonId, row.id),
    ))
    .groupBy(versionGroups.id)
    .orderBy(asc(versionGroups.id));

  const allLearnMethods = await db.select({
    id:   moveLearnMethods.id,
    slug: moveLearnMethods.slug,
    name: moveLearnMethods.name,
  }).from(moveLearnMethods).orderBy(asc(moveLearnMethods.id));

  // Pick the active version group from ?vg=… or the most populated for this pokemon
  let selectedVgId: number | null = null;
  const vgParam = url.searchParams.get('vg');
  if (vgParam) {
    const match = vgCounts.find(v => v.slug === vgParam);
    if (match) selectedVgId = match.id;
  }
  if (selectedVgId == null) {
    const best = vgCounts.filter(v => v.count > 0).sort((a, b) => b.count - a.count)[0];
    if (best) selectedVgId = best.id;
    else if (vgCounts.length > 0) selectedVgId = vgCounts[0].id;
  }

  let learnset: {
    moveId: number; moveName: string; moveSlug: string; typeId: number;
    typeName: string; typeHexColor: string | null; damageClass: string | null;
    learnMethodId: number; learnMethodName: string;
    level: number | null;
  }[] = [];
  if (selectedVgId != null) {
    learnset = await db.select({
      moveId:          pokemonMoves.moveId,
      moveName:        moves.name,
      moveSlug:        moves.slug,
      typeId:          moves.typeId,
      typeName:        types.name,
      typeHexColor:    types.hexColor,
      damageClass:     moves.damageClass,
      learnMethodId:   pokemonMoves.learnMethodId,
      learnMethodName: moveLearnMethods.name,
      level:           pokemonMoves.level,
    })
      .from(pokemonMoves)
      .innerJoin(moves, eq(pokemonMoves.moveId, moves.id))
      .innerJoin(types, eq(moves.typeId, types.id))
      .innerJoin(moveLearnMethods, eq(pokemonMoves.learnMethodId, moveLearnMethods.id))
      .where(and(eq(pokemonMoves.pokemonId, row.id), eq(pokemonMoves.versionGroupId, selectedVgId)))
      .orderBy(asc(pokemonMoves.learnMethodId), asc(pokemonMoves.level), asc(moves.name));
  }

  // Genus (English)
  const genusRow = await db.select({ genus: pokemonGenera.genus })
    .from(pokemonGenera)
    .innerJoin(languages, eq(pokemonGenera.languageId, languages.id))
    .where(and(
      eq(pokemonGenera.speciesId, row.speciesId),
      eq(languages.name, 'en')
    ))
    .limit(1);

  const genus = genusRow[0]?.genus ?? null;

  const [speciesRow] = await db.select()
    .from(pokemonSpecies)
    .where(eq(pokemonSpecies.id, row.speciesId))
    .limit(1);

  const currentEggGroups = await db.select({ id: eggGroups.id, name: eggGroups.name })
    .from(speciesEggGroups)
    .innerJoin(eggGroups, eq(speciesEggGroups.eggGroupId, eggGroups.id))
    .where(eq(speciesEggGroups.speciesId, row.speciesId));

  const allEggGroups = await db.select({ id: eggGroups.id, name: eggGroups.name })
    .from(eggGroups)
    .orderBy(asc(eggGroups.name));

  // Dex navigation
  const dexList = await db.select({ id: pokedexes.id, name: pokedexes.name, slug: pokedexes.slug })
    .from(pokedexes)
    .innerJoin(pokedexEntries, eq(pokedexEntries.pokedexId, pokedexes.id))
    .where(eq(pokedexEntries.speciesId, row.speciesId))
    .orderBy(asc(pokedexes.id));

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

    const [currentEntryRow] = await db.select({ entryNumber: pokedexEntries.entryNumber })
      .from(pokedexEntries)
      .where(and(
        eq(pokedexEntries.pokedexId, selectedDex.id),
        eq(pokedexEntries.speciesId, row.speciesId)
      ))
      .limit(1);

    if (currentEntryRow) {
      const currentEntry = currentEntryRow.entryNumber;

      const prevRows = await db.select({
        entryNumber: pokedexEntries.entryNumber,
        speciesName: pokemonSpecies.name,
        pokemonSlug: pokemon.slug,
      })
      .from(pokedexEntries)
      .innerJoin(pokemonSpecies, eq(pokedexEntries.speciesId, pokemonSpecies.id))
      .innerJoin(pokemon, eq(pokemon.id, pokemonSpecies.defaultPokemonId))
      .where(and(
        eq(pokedexEntries.pokedexId, selectedDex.id),
        lt(pokedexEntries.entryNumber, currentEntry)
      ))
      .orderBy(desc(pokedexEntries.entryNumber))
      .limit(1);

      const nextRows = await db.select({
        entryNumber: pokedexEntries.entryNumber,
        speciesName: pokemonSpecies.name,
        pokemonSlug: pokemon.slug,
      })
      .from(pokedexEntries)
      .innerJoin(pokemonSpecies, eq(pokedexEntries.speciesId, pokemonSpecies.id))
      .innerJoin(pokemon, eq(pokemon.id, pokemonSpecies.defaultPokemonId))
      .where(and(
        eq(pokedexEntries.pokedexId, selectedDex.id),
        gt(pokedexEntries.entryNumber, currentEntry)
      ))
      .orderBy(asc(pokedexEntries.entryNumber))
      .limit(1);

      nav = {
        dexList,
        selectedDex,
        currentEntry,
        prev: prevRows[0] ?? null,
        next: nextRows[0] ?? null,
      };
    }
  }

  // Version Mechanics (Flavor Texts)
  const availableVersions = await db.select({
    id: versionGroups.id,
    name: versionGroups.name,
    order: versionGroups.order,
    source: versionGroups.source
  })
  .from(versionGroups)
  .orderBy(asc(versionGroups.order));

  const flavorTexts = await db.select({
    flavorText: pokemonFlavorTexts.flavorText,
    versionGroupId: pokemonFlavorTexts.versionGroupId,
    languageId: pokemonFlavorTexts.languageId
  })
  .from(pokemonFlavorTexts)
  .innerJoin(languages, eq(pokemonFlavorTexts.languageId, languages.id))
  .where(
    and(
      eq(pokemonFlavorTexts.speciesId, row.speciesId),
      eq(languages.name, 'en')
    )
  );

  const typeEffectivenessList = await db.select({
    attackerId: typeEffectiveness.attackerId,
    defenderId: typeEffectiveness.defenderId,
    multiplier: typeEffectiveness.multiplier,
  }).from(typeEffectiveness);

  return {
    nav,
    availableVersions,
    flavorTexts,
    pokemon:    {
      ...row,
      isDefault: row.id === speciesRow?.defaultPokemonId,
    },
    types:      typeRows,
    abilities:  abilityRows,
    speciesForms,
    evolutionChain,
    chainSpecies,
    species:    speciesList,
    allTypes:   typeList,
    allAbilities: abilityList,
    predecessor,
    predecessorConditions,
    successors: successorRows,
    allPokemon,
    allItems,
    allMoves,
    vgCounts,
    allLearnMethods,
    selectedVgId,
    learnset,
    genus,
    currentSpeciesDetail: speciesRow ?? null,
    currentEggGroups,
    allEggGroups,
    typeEffectivenessList,
  };
};

// Helper to save a file upload locally inside static/sprites/
async function saveUploadedFile(file: unknown, pokemonSlug: string, suffix: string): Promise<string | null> {
  if (!file || !(file instanceof File) || file.size === 0) {
    return null;
  }

  // Ensure output directory exists inside static
  const outputDir = path.join('static', 'sprites');
  fs.mkdirSync(outputDir, { recursive: true });

  // Get extension from original file name, fallback to png
  let ext = 'png';
  if (file.name) {
    const parsedExt = path.extname(file.name).toLowerCase();
    if (parsedExt) {
      ext = parsedExt.slice(1); // remove the leading dot
    }
  }

  // Clean the file extension (alphanumeric only)
  ext = ext.replace(/[^\w]/g, '') || 'png';

  const filename = `${pokemonSlug}-${suffix}.${ext}`;
  const filePath = path.join(outputDir, filename);

  // Convert File object to Buffer and write
  const arrayBuffer = await file.arrayBuffer();
  fs.writeFileSync(filePath, Buffer.from(arrayBuffer));

  // Return the public web-accessible URL path
  return `/sprites/${filename}`;
}

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const formData = await request.formData();
    
    // Extract regular text/number fields to validate with Zod schema
    const form: Record<string, any> = {};
    for (const key of formData.keys()) {
      if (!key.endsWith('File')) {
        form[key] = formData.get(key);
      }
    }

    const parsed = UpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { updateError: parsed.error.issues[0]?.message ?? 'Inválido', values: form });
    }
    const data = parsed.data;

    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: pokemon.id })
        .from(pokemon).where(eq(pokemon.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    // Save uploaded files if any
    const defaultFile = formData.get('spriteDefaultFile');
    const shinyFile = formData.get('spriteShinyFile');
    const artworkFile = formData.get('spriteArtworkFile');
    const artworkShinyFile = formData.get('spriteArtworkShinyFile');
    const backDefaultFile = formData.get('spriteBackDefaultFile');
    const backShinyFile = formData.get('spriteBackShinyFile');

    let spriteDefault = data.spriteDefault;
    let spriteShiny = data.spriteShiny;
    let spriteArtwork = data.spriteArtwork;
    let spriteArtworkShiny = data.spriteArtworkShiny;
    let spriteBackDefault = data.spriteBackDefault;
    let spriteBackShiny = data.spriteBackShiny;

    const savedDefault = await saveUploadedFile(defaultFile, data.slug, 'default');
    if (savedDefault) spriteDefault = savedDefault;

    const savedShiny = await saveUploadedFile(shinyFile, data.slug, 'shiny');
    if (savedShiny) spriteShiny = savedShiny;

    const savedArtwork = await saveUploadedFile(artworkFile, data.slug, 'artwork');
    if (savedArtwork) spriteArtwork = savedArtwork;

    const savedArtworkShiny = await saveUploadedFile(artworkShinyFile, data.slug, 'artwork-shiny');
    if (savedArtworkShiny) spriteArtworkShiny = savedArtworkShiny;

    const savedBackDefault = await saveUploadedFile(backDefaultFile, data.slug, 'back-default');
    if (savedBackDefault) spriteBackDefault = savedBackDefault;

    const savedBackShiny = await saveUploadedFile(backShinyFile, data.slug, 'back-shiny');
    if (savedBackShiny) spriteBackShiny = savedBackShiny;

    const newRow = {
      name:          data.name,
      slug:          data.slug,
      speciesId:     data.speciesId,
      height:        data.height,
      weight:        data.weight,
      hp:            data.hp,
      atk:           data.atk,
      def:           data.def,
      spa:           data.spa,
      spd:           data.spd,
      spe:           data.spe,
      spriteDefault,
      spriteShiny,
      spriteArtwork,
      spriteArtworkShiny,
      spriteBackDefault,
      spriteBackShiny,
      source:        bumpSource(current.source),
    };

    await db.update(pokemon).set(newRow).where(eq(pokemon.id, current.id));

    await db.update(pokemonForms)
      .set({
        spriteDefault: newRow.spriteDefault,
        spriteShiny: newRow.spriteShiny,
        spriteArtwork: newRow.spriteArtwork,
        spriteArtworkShiny: newRow.spriteArtworkShiny,
        spriteBackDefault: newRow.spriteBackDefault,
        spriteBackShiny: newRow.spriteBackShiny
      })
      .where(and(eq(pokemonForms.pokemonId, current.id), eq(pokemonForms.isDefault, true)));

    // Update types
    await db.delete(pokemonTypes).where(eq(pokemonTypes.pokemonId, current.id));
    await db.insert(pokemonTypes).values({ pokemonId: current.id, typeId: data.slot1TypeId, slot: 1 });
    if (data.slot2TypeId) {
      await db.insert(pokemonTypes).values({ pokemonId: current.id, typeId: data.slot2TypeId, slot: 2 });
    }

    // Update abilities
    await db.delete(pokemonAbilities).where(eq(pokemonAbilities.pokemonId, current.id));
    await db.insert(pokemonAbilities).values({ pokemonId: current.id, abilityId: data.slot1AbilityId, slot: 1, isHidden: false });
    if (data.slot2AbilityId && data.slot2AbilityId !== data.slot1AbilityId) {
      await db.insert(pokemonAbilities).values({ pokemonId: current.id, abilityId: data.slot2AbilityId, slot: 2, isHidden: false });
    }
    if (data.hiddenAbilityId && data.hiddenAbilityId !== data.slot1AbilityId && data.hiddenAbilityId !== data.slot2AbilityId) {
      await db.insert(pokemonAbilities).values({ pokemonId: current.id, abilityId: data.hiddenAbilityId, slot: 3, isHidden: true });
    }

    // Update version-specific flavor text
    const selectedVgId = formData.get('selectedVgId') ? Number(formData.get('selectedVgId')) : null;
    const flavorText = formData.get('flavorText') != null ? String(formData.get('flavorText')).trim() : null;

    if (selectedVgId && flavorText !== null) {
      // Validate that this is a custom version group
      const [vg] = await db.select({ source: versionGroups.source })
        .from(versionGroups)
        .where(eq(versionGroups.id, selectedVgId))
        .limit(1);

      if (vg && vg.source === 'custom') {
        const [langRow] = await db.select({ id: languages.id })
          .from(languages)
          .where(eq(languages.name, 'en'))
          .limit(1);
        const languageId = langRow?.id ?? 9;

        const [existing] = await db.select()
          .from(pokemonFlavorTexts)
          .where(and(
            eq(pokemonFlavorTexts.speciesId, current.speciesId),
            eq(pokemonFlavorTexts.versionGroupId, selectedVgId),
            eq(pokemonFlavorTexts.languageId, languageId)
          ))
          .limit(1);

        if (existing) {
          await db.update(pokemonFlavorTexts)
            .set({ flavorText })
            .where(and(
              eq(pokemonFlavorTexts.speciesId, current.speciesId),
              eq(pokemonFlavorTexts.versionGroupId, selectedVgId),
              eq(pokemonFlavorTexts.languageId, languageId)
            ));
        } else {
          await db.insert(pokemonFlavorTexts)
            .values({
              speciesId: current.speciesId,
              versionGroupId: selectedVgId,
              languageId,
              flavorText,
            });
        }
      }
    }
    // Update evolution predecessor and conditions
    const predecessorIdVal = formData.get('predecessorId') ? String(formData.get('predecessorId')) : '';
    const evoTriggerVal = formData.get('evoTrigger') ? String(formData.get('evoTrigger')) : 'level-up';
    const minLevelVal = intOrNull.parse(formData.get('minLevel'));
    const minHappinessVal = intOrNull.parse(formData.get('minHappiness'));
    const timeOfDayVal = textOrNull.parse(formData.get('timeOfDay'));
    const genderVal = intOrNull.parse(formData.get('gender'));
    const locationVal = textOrNull.parse(formData.get('location'));
    const itemIdVal = intOrNull.parse(formData.get('itemId'));
    const heldItemIdVal = intOrNull.parse(formData.get('heldItemId'));
    const knownMoveIdVal = intOrNull.parse(formData.get('knownMoveId'));
    const knownMoveTypeIdVal = intOrNull.parse(formData.get('knownMoveTypeId'));

    if (!predecessorIdVal) {
      await db.update(pokemon)
        .set({ evolvesFromPokemonId: null })
        .where(eq(pokemon.id, current.id));
      await db.delete(pokemonEvolutionDetails)
        .where(eq(pokemonEvolutionDetails.pokemonId, current.id));
    } else {
      const parentId = Number(predecessorIdVal);
      await db.update(pokemon)
        .set({ evolvesFromPokemonId: parentId })
        .where(eq(pokemon.id, current.id));

      const conditionRow = {
        pokemonId:       current.id,
        trigger:         evoTriggerVal as any,
        conditions: {
          minLevel:        minLevelVal,
          itemId:          itemIdVal,
          heldItemId:      heldItemIdVal,
          knownMoveId:     knownMoveIdVal,
          knownMoveTypeId: knownMoveTypeIdVal,
          minHappiness:    minHappinessVal,
          timeOfDay:       timeOfDayVal as any,
          gender:          genderVal,
          location:        locationVal,
        },
        source:          'edited' as const,
      };

      await db.insert(pokemonEvolutionDetails).values(conditionRow).onConflictDoUpdate({
        target: pokemonEvolutionDetails.pokemonId,
        set: {
          trigger:         conditionRow.trigger,
          conditions:      conditionRow.conditions,
          source:          'edited',
        },
      });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/pokemon/${data.slug}`);
    return { saved: true };
  },

  setFormBase: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const basePokemonId = Number(form.basePokemonId);
    const formType = String(form.formType || 'other');
    if (!Number.isInteger(basePokemonId) || basePokemonId <= 0) {
      return fail(400, { evoError: 'basePokemonId inválido' });
    }
    const [current] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (basePokemonId === current.id) return fail(400, { evoError: 'Cannot be a form of itself' });
    const [base] = await db.select({ id: pokemon.id }).from(pokemon)
      .where(eq(pokemon.id, basePokemonId)).limit(1);
    if (!base) return fail(400, { evoError: 'Base Pokémon not found' });
    await db.update(pokemon)
      .set({ formOfPokemonId: basePokemonId, formType, source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));
    return { evoSaved: true };
  },

  clearFormBase: async ({ params, locals }) => {
    const [current] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    await db.update(pokemon)
      .set({ formOfPokemonId: null, formType: null, source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));
    return { evoSaved: true };
  },



  addSuccessor: async ({ request, params, locals }) => {
    const formData = await request.formData();
    const childId = intOrNull.parse(formData.get('childPokemonId'));
    if (!childId) return fail(400, { evoError: 'childPokemonId required' });

    const [cur] = await db.select({ id: pokemon.id })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!cur) throw error(404, 'Pokémon not found');

    const [child] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.id, childId)).limit(1);
    if (!child) return fail(404, { evoError: 'Target pokemon not found' });

    await db.update(pokemon)
      .set({ evolvesFromPokemonId: cur.id, source: bumpSource(child.source) })
      .where(eq(pokemon.id, childId));
    await db.insert(pokemonEvolutionDetails)
      .values({ pokemonId: childId, trigger: 'level-up', source: 'edited' })
      .onConflictDoNothing();

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', childId, child, { evolvesFromPokemonId: cur.id });
    }

    return { evoSaved: true };
  },

  addLearnsetEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = LearnsetEntrySchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { learnsetError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const [exists] = await db.select({ moveId: pokemonMoves.moveId })
      .from(pokemonMoves)
      .where(and(
        eq(pokemonMoves.pokemonId, current.id),
        eq(pokemonMoves.moveId, data.moveId),
        eq(pokemonMoves.learnMethodId, data.learnMethodId),
        eq(pokemonMoves.level, data.level),
        eq(pokemonMoves.versionGroupId, data.versionGroupId),
      )).limit(1);
    if (exists) return fail(400, { learnsetError: 'Entrada já existe nessa combinação' });

    await db.insert(pokemonMoves).values({
      pokemonId:      current.id,
      moveId:         data.moveId,
      learnMethodId:  data.learnMethodId,
      level:          data.level,
      versionGroupId: data.versionGroupId,
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, null, data);
    }

    return { learnsetSaved: true };
  },

  removeLearnsetEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = LearnsetEntrySchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { learnsetError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    await db.delete(pokemonMoves).where(and(
      eq(pokemonMoves.pokemonId, current.id),
      eq(pokemonMoves.moveId, data.moveId),
      eq(pokemonMoves.learnMethodId, data.learnMethodId),
      eq(pokemonMoves.level, data.level),
      eq(pokemonMoves.versionGroupId, data.versionGroupId),
    ));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, data, null);
    }

    return { learnsetSaved: true };
  },

  updateLearnsetEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = LearnsetUpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { learnsetError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    db.transaction((tx) => {
      tx.delete(pokemonMoves).where(and(
        eq(pokemonMoves.pokemonId, current.id),
        eq(pokemonMoves.moveId, data.oldMoveId),
        eq(pokemonMoves.learnMethodId, data.oldLearnMethodId),
        eq(pokemonMoves.level, data.oldLevel),
        eq(pokemonMoves.versionGroupId, data.oldVersionGroupId),
      )).run();
      tx.insert(pokemonMoves).values({
        pokemonId:      current.id,
        moveId:         data.newMoveId,
        learnMethodId:  data.newLearnMethodId,
        level:          data.newLevel,
        versionGroupId: data.newVersionGroupId,
      }).onConflictDoNothing().run();
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id,
        { moveId: data.oldMoveId, learnMethodId: data.oldLearnMethodId, level: data.oldLevel, versionGroupId: data.oldVersionGroupId },
        { moveId: data.newMoveId, learnMethodId: data.newLearnMethodId, level: data.newLevel, versionGroupId: data.newVersionGroupId },
      );
    }

    return { learnsetSaved: true };
  },

  clearLearnsetForVg: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const vgId = Number(form.versionGroupId);
    if (!Number.isInteger(vgId) || vgId <= 0) {
      return fail(400, { learnsetError: 'versionGroupId inválido' });
    }

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const [{ c: removed }] = await db.select({ c: count() })
      .from(pokemonMoves)
      .where(and(eq(pokemonMoves.pokemonId, current.id), eq(pokemonMoves.versionGroupId, vgId)));

    await db.delete(pokemonMoves).where(and(
      eq(pokemonMoves.pokemonId, current.id),
      eq(pokemonMoves.versionGroupId, vgId),
    ));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, { clearedVgId: vgId, removed }, null);
    }

    return { learnsetSaved: true };
  },

  copyLearnsetFromVg: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const sourceVgId = Number(form.sourceVgId);
    const targetVgId = Number(form.targetVgId);
    if (!Number.isInteger(sourceVgId) || sourceVgId <= 0 || !Number.isInteger(targetVgId) || targetVgId <= 0) {
      return fail(400, { learnsetError: 'IDs de version group inválidos' });
    }
    if (sourceVgId === targetVgId) {
      return fail(400, { learnsetError: 'Source e target precisam ser diferentes' });
    }

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const sourceRows = await db.select({
      moveId:        pokemonMoves.moveId,
      learnMethodId: pokemonMoves.learnMethodId,
      level:         pokemonMoves.level,
    })
      .from(pokemonMoves)
      .where(and(eq(pokemonMoves.pokemonId, current.id), eq(pokemonMoves.versionGroupId, sourceVgId)));

    if (sourceRows.length === 0) {
      return fail(400, { learnsetError: 'Source version group não tem nenhuma entrada' });
    }

    let copied = 0;
    db.transaction((tx) => {
      for (const r of sourceRows) {
        const result = tx.insert(pokemonMoves).values({
          pokemonId:      current.id,
          moveId:         r.moveId,
          learnMethodId:  r.learnMethodId,
          level:          r.level,
          versionGroupId: targetVgId,
        }).onConflictDoNothing().run();
        if (result.changes > 0) copied++;
      }
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, null, { copiedFromVg: sourceVgId, toVg: targetVgId, copied });
    }

    return { learnsetSaved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas pokémon edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(pokemon).set({ source: 'pokeapi' }).where(eq(pokemon.id, current.id));
    try {
      await reimportEntity('pokemon', current.pokeapiId);
    } catch (err) {
      await db.update(pokemon).set({ source: 'edited' }).where(eq(pokemon.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas pokémon custom podem ser deletados' });
    }

    await db.delete(pokemonTypes).where(eq(pokemonTypes.pokemonId, current.id));
    await db.delete(pokemonAbilities).where(eq(pokemonAbilities.pokemonId, current.id));
    await db.delete(pokemonForms).where(eq(pokemonForms.pokemonId, current.id));
    await db.delete(pokemonEvolutionDetails).where(eq(pokemonEvolutionDetails.pokemonId, current.id));
    await db.delete(pokemonMoves).where(eq(pokemonMoves.pokemonId, current.id));
    await db.delete(pokemon).where(eq(pokemon.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, null);
    }

    throw redirect(303, '/editor/pokemon');
  },
};
