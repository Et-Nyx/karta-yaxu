// ============================================================================
// Import Service — Core Data & Pokémon imports
// Extends import-service.ts with ensure* for types, abilities, moves, items,
// pokemon, species, forms, evolution chains.
// ============================================================================

import { and, eq, isNull, ne } from 'drizzle-orm';
import { db } from '$lib/db/index';
import * as schema from '$lib/db/schema/index';
import * as client from './client';
import * as mappers from './mappers';
import {
  protectedUpsert, buildIdMap, getLangNameMap, batchProcess, logImport,
  ensureGeneration, ensureVersionGroup, ensureEggGroup, ensureMoveLearnMethod,
  type ImportResult,
} from './import-service';

function emptyResult(entity: string): ImportResult {
  return { entity, total: 0, inserted: 0, updated: 0, skipped: 0, errors: [] };
}

// ── Core Data Ensure Functions ─────────────────────────────────────────────

export async function ensureType(
  pokeapiId: number,
  opts: { forceRefresh?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.types).where(eq(schema.types.pokeapiId, pokeapiId)).get();
  if (existing && !opts.forceRefresh) return existing.id;

  const data = await client.fetchType(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapType(data, genDbId, languageMap, langNameMap);

  if (existing && opts.forceRefresh) {
    await db.delete(schema.typeNames).where(eq(schema.typeNames.typeId, existing.id));
    await db.delete(schema.typeEffectiveness).where(eq(schema.typeEffectiveness.attackerId, existing.id));
  }

  const { id } = await protectedUpsert(schema.types, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.typeNames).values({ typeId: id, ...n }).onConflictDoNothing();
  }

  // Type effectiveness — needs defender type IDs resolved
  for (const eff of mapped.effectiveness) {
    const defenderDbId = await ensureType(eff.defenderPokeapiId);
    await db.insert(schema.typeEffectiveness)
      .values({ attackerId: id, defenderId: defenderDbId, multiplier: eff.multiplier })
      .onConflictDoNothing();
  }

  await logImport('type', pokeapiId, 'inserted');
  return id;
}

export async function ensureAbility(
  pokeapiId: number,
  opts: { forceRefresh?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.abilities).where(eq(schema.abilities.pokeapiId, pokeapiId)).get();
  if (existing && !opts.forceRefresh) return existing.id;

  const data = await client.fetchAbility(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const vgMap = await buildIdMap(schema.versionGroups);
  const mapped = mappers.mapAbility(data, genDbId, languageMap, langNameMap, vgMap);

  if (existing && opts.forceRefresh) {
    await db.delete(schema.abilityNames).where(eq(schema.abilityNames.abilityId, existing.id));
    await db.delete(schema.abilityFlavorTexts).where(eq(schema.abilityFlavorTexts.abilityId, existing.id));
    await db.delete(schema.abilityEffects).where(eq(schema.abilityEffects.abilityId, existing.id));
  }

  const { id } = await protectedUpsert(schema.abilities, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.abilityNames).values({ abilityId: id, ...n }).onConflictDoNothing();
  }
  for (const ft of mapped.flavorTexts) {
    await db.insert(schema.abilityFlavorTexts).values({ abilityId: id, ...ft }).onConflictDoNothing();
  }
  for (const e of mapped.effects) {
    await db.insert(schema.abilityEffects).values({ abilityId: id, ...e }).onConflictDoNothing();
  }

  await logImport('ability', pokeapiId, 'inserted');
  return id;
}

export async function ensureMove(
  pokeapiId: number,
  opts: { forceRefresh?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.moves).where(eq(schema.moves.pokeapiId, pokeapiId)).get();
  if (existing && !opts.forceRefresh) return existing.id;

  const data = await client.fetchMove(pokeapiId);
  const typePokeapiId = client.extractIdFromUrl(data.type.url);
  const typeDbId = await ensureType(typePokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const vgMap = await buildIdMap(schema.versionGroups);
  const mapped = mappers.mapMove(data, typeDbId, genDbId, languageMap, langNameMap, vgMap);

  if (existing && opts.forceRefresh) {
    await db.delete(schema.moveNames).where(eq(schema.moveNames.moveId, existing.id));
    await db.delete(schema.moveFlavorTexts).where(eq(schema.moveFlavorTexts.moveId, existing.id));
    await db.delete(schema.moveEffects).where(eq(schema.moveEffects.moveId, existing.id));
  }

  const { id } = await protectedUpsert(schema.moves, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.moveNames).values({ moveId: id, ...n }).onConflictDoNothing();
  }
  for (const ft of mapped.flavorTexts) {
    await db.insert(schema.moveFlavorTexts).values({ moveId: id, ...ft }).onConflictDoNothing();
  }
  for (const e of mapped.effects) {
    await db.insert(schema.moveEffects).values({ moveId: id, ...e }).onConflictDoNothing();
  }

  await logImport('move', pokeapiId, 'inserted');
  return id;
}

export async function ensureItem(
  pokeapiId: number,
  opts: { forceRefresh?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.items).where(eq(schema.items.pokeapiId, pokeapiId)).get();
  if (existing && !opts.forceRefresh) return existing.id;

  const data = await client.fetchItem(pokeapiId);
  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const vgMap = await buildIdMap(schema.versionGroups);
  const mapped = mappers.mapItem(data, languageMap, langNameMap, vgMap);

  if (existing && opts.forceRefresh) {
    await db.delete(schema.itemNames).where(eq(schema.itemNames.itemId, existing.id));
    await db.delete(schema.itemFlavorTexts).where(eq(schema.itemFlavorTexts.itemId, existing.id));
    await db.delete(schema.itemEffects).where(eq(schema.itemEffects.itemId, existing.id));
  }

  const { id } = await protectedUpsert(schema.items, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.itemNames).values({ itemId: id, ...n }).onConflictDoNothing();
  }
  for (const ft of mapped.flavorTexts) {
    await db.insert(schema.itemFlavorTexts).values({ itemId: id, ...ft }).onConflictDoNothing();
  }
  for (const e of mapped.effects) {
    await db.insert(schema.itemEffects).values({ itemId: id, ...e }).onConflictDoNothing();
  }

  await logImport('item', pokeapiId, 'inserted');
  return id;
}

// ── Pokémon Ensure Functions ───────────────────────────────────────────────

/**
 * Ensures all species in a PokeAPI evolution chain exist. Does NOT populate the linked-list
 * evolution edges — that happens later via `populateChainLinkedList`, once the corresponding
 * default pokemon rows have been imported (chicken-and-egg: edges need pokemon ids).
 */
export async function ensureEvolutionChain(
  pokeapiId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<void> {
  const data = await client.fetchEvolutionChain(pokeapiId);
  const speciesIds = mappers.extractSpeciesFromChain(data.chain);
  for (const spId of speciesIds) {
    try {
      await ensurePokemonSpeciesBase(spId, opts);
    } catch {
      // skip species that fail
    }
  }
  await logImport('evolution_chain', pokeapiId, 'inserted');
}

/**
 * Walks a PokeAPI chain and populates pokemon.evolves_from_pokemon_id +
 * pokemon_evolution_details for each edge whose both endpoint default pokemon
 * already exist. Respects source protection: only writes when existing source is
 * 'pokeapi' (or row is missing). Idempotent — safe to re-run.
 */
export async function populateChainLinkedList(chainPokeapiId: number): Promise<void> {
  const chain = await client.fetchEvolutionChain(chainPokeapiId);
  const edges = mappers.flattenEvolutionChain(chain.chain);
  const processedTargets = new Set<number>();
  const REGIONAL_SUFFIXES = ['alola', 'galar', 'hisui', 'paldea'];

  for (const edge of edges) {
    try {
      // Find predecessor & successor species
      const [fromSp] = await db.select({ id: schema.pokemonSpecies.id, defaultPokemonId: schema.pokemonSpecies.defaultPokemonId })
        .from(schema.pokemonSpecies)
        .where(eq(schema.pokemonSpecies.pokeapiId, edge.fromSpeciesPokeapiId)).limit(1);

      const [toSp] = await db.select({ id: schema.pokemonSpecies.id, defaultPokemonId: schema.pokemonSpecies.defaultPokemonId })
        .from(schema.pokemonSpecies)
        .where(eq(schema.pokemonSpecies.pokeapiId, edge.toSpeciesPokeapiId)).limit(1);

      if (!fromSp || !toSp) continue;

      // Find all pokemon variants for these species
      const fromPkRows = await db.select().from(schema.pokemon).where(eq(schema.pokemon.speciesId, fromSp.id));
      const toPkRows = await db.select().from(schema.pokemon).where(eq(schema.pokemon.speciesId, toSp.id));

      const defaultFromPk = fromPkRows.find(p => p.id === fromSp.defaultPokemonId) ?? fromPkRows[0];
      const defaultToPk = toPkRows.find(p => p.id === toSp.defaultPokemonId) ?? toPkRows[0];

      if (!defaultFromPk || !defaultToPk) continue;

      // Resolve item/move/type FKs for this edge
      let itemDbId:          number | null = null;
      if (edge.itemPokeapiId)          itemDbId          = await ensureItem(edge.itemPokeapiId);
      let heldItemDbId:      number | null = null;
      if (edge.heldItemPokeapiId)      heldItemDbId      = await ensureItem(edge.heldItemPokeapiId);
      let knownMoveDbId:     number | null = null;
      if (edge.knownMovePokeapiId)     knownMoveDbId     = await ensureMove(edge.knownMovePokeapiId);
      let knownMoveTypeDbId: number | null = null;
      if (edge.knownMoveTypePokeapiId) knownMoveTypeDbId = await ensureType(edge.knownMoveTypePokeapiId);

      // Resolve party requirements FKs
      let partySpeciesDbId: number | null = null;
      if (edge.partySpeciesPokeapiId) {
        const pk = await defaultPokemonForSpeciesPokeapiId(edge.partySpeciesPokeapiId);
        partySpeciesDbId = pk?.id ?? null;
      }
      let partyTypeDbId: number | null = null;
      if (edge.partyTypePokeapiId) {
        partyTypeDbId = await ensureType(edge.partyTypePokeapiId);
      }

      // Determine edge suffix
      let edgeSuffix: string | null = null;
      let itemSlug = '';
      if (itemDbId) {
        const [it] = await db.select({ slug: schema.items.slug }).from(schema.items).where(eq(schema.items.id, itemDbId)).limit(1);
        itemSlug = it?.slug ?? '';
      }
      let heldItemSlug = '';
      if (heldItemDbId) {
        const [it] = await db.select({ slug: schema.items.slug }).from(schema.items).where(eq(schema.items.id, heldItemDbId)).limit(1);
        heldItemSlug = it?.slug ?? '';
      }

      for (const suffix of REGIONAL_SUFFIXES) {
        if (itemSlug.includes(suffix) || heldItemSlug.includes(suffix) || (edge.location && edge.location.includes(suffix))) {
          edgeSuffix = suffix;
          break;
        }
      }

      // Special cases for generic items triggering regional forms
      if (!edgeSuffix && itemSlug === 'ice-stone') {
        const hasAlola = toPkRows.some(p => p.slug.endsWith('-alola'));
        const hasGalar = toPkRows.some(p => p.slug.endsWith('-galar'));
        if (hasAlola) edgeSuffix = 'alola';
        else if (hasGalar) edgeSuffix = 'galar';
      }
      if (!edgeSuffix && itemSlug === 'leaf-stone') {
        const hasAlola = toPkRows.some(p => p.slug.endsWith('-alola'));
        const hasHisui = toPkRows.some(p => p.slug.endsWith('-hisui'));
        if (hasAlola) edgeSuffix = 'alola';
        else if (hasHisui) edgeSuffix = 'hisui';
      }
      if (!edgeSuffix && itemSlug === 'fire-stone') {
        if (toPkRows.some(p => p.slug.endsWith('-hisui'))) edgeSuffix = 'hisui';
      }
      if (!edgeSuffix && itemSlug === 'sun-stone') {
        if (toPkRows.some(p => p.slug.endsWith('-hisui'))) edgeSuffix = 'hisui';
      }

      // Helper to identify regional suffixes for both regular forms and regional-only species
      function getPkSuffix(slug: string): string | null {
        const REGIONAL_ONLY_SPECIES: Record<string, string> = {
          'mr-rime': 'galar',
          'obstagoon': 'galar',
          'perrserker': 'galar',
          'sirfetchd': 'galar',
          'cursola': 'galar',
          'runerigus': 'galar',
          'sneasler': 'hisui',
          'overqwil': 'hisui',
          'basculegion': 'hisui',
          'wyrdeer': 'hisui',
          'kleavor': 'hisui',
          'ursaluna': 'hisui'
        };
        for (const [k, v] of Object.entries(REGIONAL_ONLY_SPECIES)) {
          if (slug === k || slug.startsWith(k + '-')) return v;
        }
        for (const suffix of REGIONAL_SUFFIXES) {
          if (slug.endsWith('-' + suffix) || slug.includes('-' + suffix + '-')) {
            return suffix;
          }
        }
        return null;
      }

      // Construct matched pairs of predecessor -> successor variety
      const matchedPairs: Array<{ from: typeof defaultFromPk; to: typeof defaultToPk }> = [];

      for (const toPk of toPkRows) {
        // Skip mega and gmax forms so they do not participate in the Evolution Chain
        const isMegaOrGmax = toPk.slug.endsWith('-mega') ||
                             toPk.slug.endsWith('-mega-x') ||
                             toPk.slug.endsWith('-mega-y') ||
                             toPk.slug.endsWith('-gmax') ||
                             toPk.slug.includes('-mega-') ||
                             toPk.slug.includes('-gmax-');
        if (isMegaOrGmax) continue;

        const toSuffix = getPkSuffix(toPk.slug);
        const hasMatchingPredecessor = fromPkRows.some(p => p.slug.endsWith('-' + toSuffix) || p.slug.includes('-' + toSuffix + '-'));

        if (toSuffix === edgeSuffix || (edgeSuffix === null && !hasMatchingPredecessor)) {
          if (toSuffix) {
            // Successor is regional. Look for predecessor variety with matching suffix
            const matchingFromPk = fromPkRows.find(p => p.slug.endsWith('-' + toSuffix) || p.slug.includes('-' + toSuffix + '-'));
            if (matchingFromPk) {
              matchedPairs.push({ from: matchingFromPk, to: toPk });
            } else {
              // Fallback to default predecessor (e.g. standard Pikachu -> Alolan Raichu)
              matchedPairs.push({ from: defaultFromPk, to: toPk });
            }
          } else {
            // Successor is standard. Look for standard predecessor variety
            const standardFromPk = fromPkRows.find(p => {
              const suffix = getPkSuffix(p.slug);
              return suffix === null;
            }) ?? defaultFromPk;
            matchedPairs.push({ from: standardFromPk, to: toPk });
          }
        }
      }

      // Upsert matched pairs to database
      for (const pair of matchedPairs) {
        if (processedTargets.has(pair.to.id)) continue;

        // Set the parent pointer only if it's currently null (don't overwrite editor edits)
        const [child] = await db.select({
          parent: schema.pokemon.evolvesFromPokemonId,
        }).from(schema.pokemon).where(eq(schema.pokemon.id, pair.to.id)).limit(1);
        if (child?.parent == null) {
          await db.update(schema.pokemon)
            .set({ evolvesFromPokemonId: pair.from.id })
            .where(eq(schema.pokemon.id, pair.to.id));
        }

        // Source-protected upsert of conditions row
        const [existingCond] = await db.select({ source: schema.pokemonEvolutionDetails.source })
          .from(schema.pokemonEvolutionDetails)
          .where(eq(schema.pokemonEvolutionDetails.pokemonId, pair.to.id)).limit(1);

        if (!existingCond || existingCond.source === 'pokeapi') {
          const conditions: schema.EvolutionConditions = {
            minLevel:        edge.minLevel ?? null,
            itemId:          itemDbId,
            heldItemId:      heldItemDbId,
            knownMoveId:     knownMoveDbId,
            knownMoveTypeId: knownMoveTypeDbId,
            minHappiness:    edge.minHappiness ?? null,
            timeOfDay:       (edge.timeOfDay as any) ?? null,
            gender:          edge.gender ?? null,
            location:        edge.location ?? null,
            partySpeciesId:  partySpeciesDbId,
            partyTypeId:     partyTypeDbId,
            relativePhysicalStats: edge.relativePhysicalStats ?? null,
            needsOverworldRain: edge.needsOverworldRain || null,
            turnUpsideDown: edge.turnUpsideDown || null,
          };
          const payload = {
            pokemonId:  pair.to.id,
            trigger:    edge.trigger as any,
            conditions: conditions,
            source:     'pokeapi' as const,
          };
          await db.insert(schema.pokemonEvolutionDetails).values(payload).onConflictDoUpdate({
            target: schema.pokemonEvolutionDetails.pokemonId,
            set: {
              trigger:    payload.trigger,
              conditions: payload.conditions,
              source:     'pokeapi',
            },
          });
          processedTargets.add(pair.to.id);
        } else {
          processedTargets.add(pair.to.id);
        }
      }
    } catch (err) {
      console.error(err);
      // skip edges with unresolvable dependencies (rare items/moves/types)
    }
  }
}

async function defaultPokemonForSpeciesPokeapiId(speciesPokeapiId: number): Promise<{ id: number } | null> {
  const [sp] = await db.select({ defaultPokemonId: schema.pokemonSpecies.defaultPokemonId })
    .from(schema.pokemonSpecies)
    .where(eq(schema.pokemonSpecies.pokeapiId, speciesPokeapiId)).limit(1);
  if (!sp || sp.defaultPokemonId == null) return null;
  return { id: sp.defaultPokemonId };
}

/**
 * Ensures a PokemonSpecies exists. This is the "base" version that inserts the
 * species record without requiring the full evolution chain to be resolved yet.
 * Used internally by ensureEvolutionChain to break circular deps.
 */
async function ensurePokemonSpeciesBase(
  pokeapiId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.pokeapiId, pokeapiId)).get();

  if (existing && existing.source === 'edited' && !opts.overwriteEdited) {
    console.log(`  🛡️ Species "${existing.name}" has manual edits. Skipping refresh.`);
    return existing.id;
  }

  // Always fetch to upsert flavor texts, even if species already exists
  const data = await client.fetchPokemonSpecies(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  // Handle evolvesFrom — recursively ensure the pre-evolution (do NOT propagate forceRefresh)
  if (data.evolves_from_species) {
    const fromPokeapiId = client.extractIdFromUrl(data.evolves_from_species.url);
    await ensurePokemonSpeciesBase(fromPokeapiId, { overwriteEdited: opts.overwriteEdited });
  }

  // Ensure egg groups
  for (const eg of data.egg_groups) {
    await ensureEggGroup(client.extractIdFromUrl(eg.url));
  }

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();

  // Build version -> versionGroup map from DB
  const versions = await db.select({ pokeapiId: schema.versions.pokeapiId, versionGroupId: schema.versions.versionGroupId }).from(schema.versions);
  const versionToVgMap = new Map<number, number>();
  for (const v of versions) {
    if (v.pokeapiId !== null) versionToVgMap.set(v.pokeapiId, v.versionGroupId);
  }

  const mapped = mappers.mapPokemonSpecies(data, genDbId, languageMap, langNameMap, versionToVgMap);

  // If species already exists AND we're not force-refreshing, only upsert flavor texts and return early
  if (existing && !opts.forceRefresh) {
    for (const ft of mapped.flavorTexts) {
      await db.insert(schema.pokemonFlavorTexts).values({
        speciesId: existing.id,
        versionGroupId: ft.versionGroupId,
        languageId: ft.languageId,
        flavorText: ft.flavorText,
      }).onConflictDoNothing();
    }
    return existing.id;
  }

  // Force-refresh path: wipe owned sub-rows for this species so re-import is a true reset
  if (existing && opts.forceRefresh) {
    await db.delete(schema.pokemonFlavorTexts).where(eq(schema.pokemonFlavorTexts.speciesId, existing.id));
    await db.delete(schema.pokemonGenera).where(eq(schema.pokemonGenera.speciesId, existing.id));
    await db.delete(schema.speciesEggGroups).where(eq(schema.speciesEggGroups.speciesId, existing.id));
  }

  const { id } = await protectedUpsert(schema.pokemonSpecies, mapped.main, pokeapiId);

  for (const g of mapped.genera) {
    await db.insert(schema.pokemonGenera).values({ speciesId: id, ...g }).onConflictDoNothing();
  }

  // Link egg groups
  for (const egPokeapiId of mapped.eggGroupPokeapiIds) {
    const egDbId = await ensureEggGroup(egPokeapiId);
    await db.insert(schema.speciesEggGroups).values({ speciesId: id, eggGroupId: egDbId }).onConflictDoNothing();
  }

  // Insert flavor texts
  for (const ft of mapped.flavorTexts) {
    await db.insert(schema.pokemonFlavorTexts).values({
      speciesId: id,
      versionGroupId: ft.versionGroupId,
      languageId: ft.languageId,
      flavorText: ft.flavorText,
    }).onConflictDoNothing();
  }

  await logImport('pokemon_species', pokeapiId, 'inserted');
  return id;
}

/**
 * Public entry point: ensures a PokemonSpecies and its full evolution chain.
 */
export async function ensurePokemonSpecies(
  pokeapiId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.pokeapiId, pokeapiId)).get();
  if (existing && existing.source === 'edited' && !opts.overwriteEdited) {
    console.log(`  🛡️ Species "${existing.name}" has manual edits. Skipping refresh.`);
    return existing.id;
  }
  if (existing && !opts.forceRefresh) return existing.id;

  if (existing && opts.forceRefresh) {
    // Re-import species data in place; do NOT recursively refresh the evolution chain
    return await ensurePokemonSpeciesBase(pokeapiId, { forceRefresh: true, overwriteEdited: opts.overwriteEdited });
  }

  // No existing row — go through the chain-import path which creates it
  const data = await client.fetchPokemonSpecies(pokeapiId);
  const chainPokeapiId = client.extractIdFromUrl(data.evolution_chain.url);
  await ensureEvolutionChain(chainPokeapiId, opts);

  const species = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.pokeapiId, pokeapiId)).get();
  if (!species) throw new Error(`Failed to import species ${pokeapiId} via evolution chain`);
  return species.id;
}

/**
 * Resolve the database ID of the default/base Pokémon variety for a given species.
 * Used to inherit missing attributes (types, abilities, stats) from the base form
 * when PokeAPI returns incomplete data for alternate varieties (e.g. custom Megas).
 */
async function getBaseFormPokemonId(speciesDbId: number): Promise<number | null> {
  const row = await db.select({ defaultPokemonId: schema.pokemonSpecies.defaultPokemonId })
    .from(schema.pokemonSpecies)
    .where(eq(schema.pokemonSpecies.id, speciesDbId))
    .get();
  return row?.defaultPokemonId ?? null;
}

export async function ensurePokemon(
  pokeapiId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.pokemon).where(eq(schema.pokemon.pokeapiId, pokeapiId)).get();

  if (existing && existing.source === 'edited' && !opts.overwriteEdited) {
    console.log(`  🛡️ Pokémon "${existing.name}" has manual edits. Skipping refresh.`);
    return existing.id;
  }

  const data = await client.fetchPokemon(pokeapiId);
  const speciesPokeapiId = client.extractIdFromUrl(data.species.url);
  const speciesDbId = await ensurePokemonSpecies(speciesPokeapiId, opts);
  const mapped = mappers.mapPokemon(data, speciesDbId);

  let pokemonDbId: number;

  // Force-refresh path: wipe owned sub-rows for this pokemon so re-import is a true reset.
  // pokemon_forms preserves custom-source rows (no PokeAPI counterpart).
  // pokemon_evolution_details also gets cleared (preserving custom rows), then repopulated
  // by populateChainLinkedList at the end.
  if (existing && opts.forceRefresh) {
    pokemonDbId = existing.id;
    await db.delete(schema.pokemonTypes).where(eq(schema.pokemonTypes.pokemonId, pokemonDbId));
    await db.delete(schema.pokemonAbilities).where(eq(schema.pokemonAbilities.pokemonId, pokemonDbId));
    await db.delete(schema.pokemonStats).where(eq(schema.pokemonStats.pokemonId, pokemonDbId));
    await db.delete(schema.pokemonMoves).where(eq(schema.pokemonMoves.pokemonId, pokemonDbId));
    await db.delete(schema.pokemonForms).where(and(
      eq(schema.pokemonForms.pokemonId, pokemonDbId),
      ne(schema.pokemonForms.source, 'custom'),
    ));
    const [existingCond] = await db.select({ source: schema.pokemonEvolutionDetails.source })
      .from(schema.pokemonEvolutionDetails)
      .where(eq(schema.pokemonEvolutionDetails.pokemonId, pokemonDbId)).limit(1);
    if (!existingCond || existingCond.source !== 'custom') {
      await db.delete(schema.pokemonEvolutionDetails).where(eq(schema.pokemonEvolutionDetails.pokemonId, pokemonDbId));
      await db.update(schema.pokemon).set({ evolvesFromPokemonId: null }).where(eq(schema.pokemon.id, pokemonDbId));
    }
  }

  if (!existing || opts.forceRefresh) {
    const { id } = await protectedUpsert(schema.pokemon, mapped.main, pokeapiId);
    pokemonDbId = id;

    if (mapped.isDefault) {
      await db.update(schema.pokemonSpecies)
        .set({ defaultPokemonId: pokemonDbId })
        .where(eq(schema.pokemonSpecies.id, speciesDbId));
    }

    // Types — fallback to base form if PokeAPI data is empty
    if (mapped.types.length > 0) {
      for (const t of mapped.types) {
        const typeDbId = await ensureType(t.typePokeapiId);
        await db.insert(schema.pokemonTypes).values({ pokemonId: pokemonDbId, typeId: typeDbId, slot: t.slot }).onConflictDoNothing();
      }
    } else {
      const basePk = await getBaseFormPokemonId(speciesDbId);
      if (basePk !== null) {
        const baseTypes = await db.select().from(schema.pokemonTypes).where(eq(schema.pokemonTypes.pokemonId, basePk));
        for (const bt of baseTypes) {
          await db.insert(schema.pokemonTypes).values({ pokemonId: pokemonDbId, typeId: bt.typeId, slot: bt.slot }).onConflictDoNothing();
        }
        console.log(`  ⚠️ No types from PokeAPI for "${mapped.main.slug}" — inherited ${baseTypes.length} type(s) from base form.`);
      }
    }

    // Abilities — fallback to base form if PokeAPI data is empty
    if (mapped.abilities.length > 0) {
      for (const a of mapped.abilities) {
        const abilityDbId = await ensureAbility(a.abilityPokeapiId);
        await db.insert(schema.pokemonAbilities).values({ pokemonId: pokemonDbId, abilityId: abilityDbId, slot: a.slot, isHidden: a.isHidden }).onConflictDoNothing();
      }
    } else {
      const basePk = await getBaseFormPokemonId(speciesDbId);
      if (basePk !== null) {
        const baseAbilities = await db.select().from(schema.pokemonAbilities).where(eq(schema.pokemonAbilities.pokemonId, basePk));
        for (const ba of baseAbilities) {
          await db.insert(schema.pokemonAbilities).values({ pokemonId: pokemonDbId, abilityId: ba.abilityId, slot: ba.slot, isHidden: ba.isHidden }).onConflictDoNothing();
        }
        console.log(`  ⚠️ No abilities from PokeAPI for "${mapped.main.slug}" — inherited ${baseAbilities.length} ability/ies from base form.`);
      }
    }

    // Stats — fallback to base form if all stats are zero
    const allStatsZero = mapped.stats.every(s => s.baseStat === 0);
    if (mapped.stats.length > 0 && !allStatsZero) {
      for (const s of mapped.stats) {
        await db.insert(schema.pokemonStats).values({ pokemonId: pokemonDbId, ...s }).onConflictDoNothing();
      }
    } else {
      const basePk = await getBaseFormPokemonId(speciesDbId);
      if (basePk !== null) {
        const baseStats = await db.select().from(schema.pokemonStats).where(eq(schema.pokemonStats.pokemonId, basePk));
        for (const bs of baseStats) {
          await db.insert(schema.pokemonStats).values({ pokemonId: pokemonDbId, statName: bs.statName, baseStat: bs.baseStat, effort: bs.effort }).onConflictDoNothing();
        }
        console.log(`  ⚠️ Stats missing/zero from PokeAPI for "${mapped.main.slug}" — inherited from base form.`);
      }
    }

    // Forms (force-refresh propagates so existing form rows get updated, not skipped)
    for (const formRef of data.forms) {
      const formPokeapiId = client.extractIdFromUrl(formRef.url);
      await ensurePokemonForm(formPokeapiId, pokemonDbId, speciesDbId, opts);
    }

    if (!existing) await logImport('pokemon', pokeapiId, 'inserted');
  } else {
    pokemonDbId = existing.id;
  }

  // Always re-process moves — idempotent via onConflictDoNothing.
  // This ensures learnsets added after the initial import (e.g. FRLG) are picked up on reimport.
  for (const m of mapped.moves) {
    try {
      const vgDbId = await ensureVersionGroup(m.versionGroupPokeapiId);
      const moveDbId = await ensureMove(m.movePokeapiId);
      const learnMethodDbId = await ensureMoveLearnMethod(m.learnMethodPokeapiId);

      await db.insert(schema.pokemonMoves).values({
        pokemonId: pokemonDbId,
        moveId: moveDbId,
        learnMethodId: learnMethodDbId,
        level: m.level,
        versionGroupId: vgDbId,
      }).onConflictDoNothing();
    } catch {
      // Skip individual moves that fail (e.g. shadow moves)
    }
  }

  // Force-refresh: repopulate evolution linked-list for this pokemon's chain
  if (opts.forceRefresh) {
    try {
      const speciesData = await client.fetchPokemonSpecies(speciesPokeapiId);
      const chainPokeapiId = client.extractIdFromUrl(speciesData.evolution_chain.url);
      await populateChainLinkedList(chainPokeapiId);
    } catch {
      // chain repopulation is best-effort; don't fail the whole re-source
    }
  }

  return pokemonDbId;
}

async function ensurePokemonForm(
  pokeapiId: number,
  pokemonDbId: number,
  speciesDbId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.pokemonForms).where(eq(schema.pokemonForms.pokeapiId, pokeapiId)).get();
  if (existing && existing.source === 'edited' && !opts.overwriteEdited) {
    console.log(`  🛡️ Form "${existing.formName}" has manual edits. Skipping refresh.`);
    return existing.id;
  }
  if (existing && !opts.forceRefresh) return existing.id;

  const data = await client.fetchPokemonForm(pokeapiId);
  const mapped = mappers.mapPokemonForm(data, pokemonDbId, speciesDbId);

  if (existing) {
    await db.update(schema.pokemonForms).set(mapped.main).where(eq(schema.pokemonForms.id, existing.id));
    return existing.id;
  }

  const result = await db.insert(schema.pokemonForms).values(mapped.main).returning({ id: schema.pokemonForms.id });
  await logImport('pokemon_form', pokeapiId, 'inserted');
  return result[0]!.id;
}

// ── Batch Imports (Core Data & Pokémon) ────────────────────────────────────

export async function importAllTypes(): Promise<ImportResult> {
  const result = emptyResult('types');
  const list = await client.fetchList('type');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      const pid = client.extractIdFromUrl(item.url);
      if (pid > 10000) continue; // skip shadow/unknown/stellar
      await ensureType(pid);
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllAbilities(): Promise<ImportResult> {
  const result = emptyResult('abilities');
  const list = await client.fetchList('ability');
  result.total = list.results.length;

  await batchProcess(list.results, async (item) => {
    try {
      await ensureAbility(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }, 5);
  return result;
}

export async function importAllMoves(): Promise<ImportResult> {
  const result = emptyResult('moves');
  const list = await client.fetchList('move');
  // Filter out shadow/unknown moves (id > 10000)
  const filtered = list.results.filter(item => client.extractIdFromUrl(item.url) <= 10000);
  result.total = filtered.length;

  await batchProcess(filtered, async (item) => {
    try {
      await ensureMove(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }, 5);
  return result;
}

export async function importAllItems(): Promise<ImportResult> {
  const result = emptyResult('items');
  const list = await client.fetchList('item');
  result.total = list.results.length;

  await batchProcess(list.results, async (item) => {
    try {
      await ensureItem(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }, 5);
  return result;
}

/** Import a single Pokémon by PokeAPI ID with full dependency resolution */
export async function importPokemonById(
  pokeapiId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<number> {
  console.log(`🔴 Importing Pokémon #${pokeapiId}...`);
  const dbId = await ensurePokemon(pokeapiId, opts);

  // Find the chain via PokeAPI (cached fetch — no DB roundtrip needed)
  try {
    const data = await client.fetchPokemon(pokeapiId);
    const speciesPokeapiId = client.extractIdFromUrl(data.species.url);
    const speciesData = await client.fetchPokemonSpecies(speciesPokeapiId);
    const chainPokeapiId = client.extractIdFromUrl(speciesData.evolution_chain.url);

    const chain = await client.fetchEvolutionChain(chainPokeapiId);
    const speciesPokeapiIds = mappers.extractSpeciesFromChain(chain.chain);

    // Import each family member's varieties
    for (const spPokeapiId of speciesPokeapiIds) {
      try {
        const spData = await client.fetchPokemonSpecies(spPokeapiId);
        for (const variety of spData.varieties) {
          if (isUnwantedVariety(variety.pokemon.name)) continue;
          const varietyPokeapiId = client.extractIdFromUrl(variety.pokemon.url);
          console.log(`  ↪ Also importing family variety ${variety.pokemon.name} (#${varietyPokeapiId})...`);
          await ensurePokemon(varietyPokeapiId, opts);
        }
      } catch (e) {
        console.warn(`  ↪ Family member species #${spPokeapiId} failed: ${(e as Error).message}`);
      }
    }

    // All family pokemon now exist → populate the linked-list evolution edges
    await populateChainLinkedList(chainPokeapiId);
  } catch (e) {
    console.warn(`  Chain processing for #${pokeapiId} skipped: ${(e as Error).message}`);
  }

  console.log(`✅ Pokémon #${pokeapiId} (and family) imported (db id: ${dbId})`);
  return dbId;
}

/**
 * Computes and stores a Regional Dex order on pokemon_species.
 * Families are sorted by the base species' National Dex number.
 * Within a family, members are ordered by DFS pre-order (base → evolutions),
 * with siblings sorted by dexNumber.
 * Run after any Pokemon import to keep ordering up-to-date.
 */


// Cosmetic/event-only variety name patterns irrelevant for a ROM hack wiki.
const SKIP_VARIETY_PATTERNS = [
  '-totem',                                              // Totem-sized (Sun/Moon)
  '-cap',                                                // Ash's hat Pikachus
  '-cosplay', '-rock-star', '-belle', '-pop-star', '-phd', '-libre', // OR/AS cosplay
  '-starter',                                            // Let's Go partner Pikachu/Eevee
];

function isUnwantedVariety(name: string): boolean {
  return SKIP_VARIETY_PATTERNS.some(p => name.includes(p));
}

// ── By-Region Batch Imports ────────────────────────────────────────────────

async function fetchGenerationForRegion(regionPokeapiId: number) {
  const region = await client.fetchRegion(regionPokeapiId);
  if (!region.main_generation) throw new Error('Region has no main generation');
  const genId = client.extractIdFromUrl(region.main_generation.url);
  return client.fetchGeneration(genId);
}

export async function importTypesByRegion(regionPokeapiId: number): Promise<ImportResult> {
  const result = emptyResult('types');
  const gen = await fetchGenerationForRegion(regionPokeapiId);
  result.total = gen.types.length;
  for (const ref of gen.types) {
    const id = client.extractIdFromUrl(ref.url);
    try { await ensureType(id); result.inserted++; }
    catch (e) { result.errors.push({ id, error: (e as Error).message }); }
  }
  return result;
}

export async function importAbilitiesByRegion(regionPokeapiId: number): Promise<ImportResult> {
  const result = emptyResult('abilities');
  const gen = await fetchGenerationForRegion(regionPokeapiId);
  result.total = gen.abilities.length;
  await batchProcess(gen.abilities, async (ref) => {
    const id = client.extractIdFromUrl(ref.url);
    try { await ensureAbility(id); result.inserted++; }
    catch (e) { result.errors.push({ id, error: (e as Error).message }); }
  }, 5);
  return result;
}

export async function importMovesByRegion(regionPokeapiId: number): Promise<ImportResult> {
  const result = emptyResult('moves');
  const gen = await fetchGenerationForRegion(regionPokeapiId);
  result.total = gen.moves.length;
  await batchProcess(gen.moves, async (ref) => {
    const id = client.extractIdFromUrl(ref.url);
    try { await ensureMove(id); result.inserted++; }
    catch (e) { result.errors.push({ id, error: (e as Error).message }); }
  }, 5);
  return result;
}

export async function importPokemonByRegion(
  regionPokeapiId: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<ImportResult> {
  const result = emptyResult('pokemon');
  const gen = await fetchGenerationForRegion(regionPokeapiId);
  result.total = gen.pokemon_species.length;
  for (const ref of gen.pokemon_species) {
    const id = client.extractIdFromUrl(ref.url);
    try { await ensurePokemon(id, opts); result.inserted++; }
    catch (e) { result.errors.push({ id, error: (e as Error).message }); }
  }
  return result;
}

/** Import a range of Pokémon (e.g. 1-151 for Gen 1).
 *  Uses importPokemonById so evo chain members and cross-gen evolutions are included. */
export async function importPokemonByRange(
  from: number,
  to: number,
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<ImportResult> {
  const result = emptyResult('pokemon');
  result.total = to - from + 1;

  for (let id = from; id <= to; id++) {
    try {
      await importPokemonById(id, opts);
      result.inserted++;
      console.log(`  ✅ #${id} done (${result.inserted}/${result.total})`);
    } catch (e) {
      result.errors.push({ id, error: (e as Error).message });
      console.error(`  ❌ #${id} failed: ${(e as Error).message}`);
    }
  }
  return result;
}
