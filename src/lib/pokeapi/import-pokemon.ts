// ============================================================================
// Import Service — Core Data & Pokémon imports
// Extends import-service.ts with ensure* for types, abilities, moves, items,
// pokemon, species, forms, evolution chains.
// ============================================================================

import { eq } from 'drizzle-orm';
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

export async function ensureType(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.types).where(eq(schema.types.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchType(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapType(data, genDbId, languageMap, langNameMap);
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

export async function ensureAbility(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.abilities).where(eq(schema.abilities.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchAbility(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const vgMap = await buildIdMap(schema.versionGroups);
  const mapped = mappers.mapAbility(data, genDbId, languageMap, langNameMap, vgMap);
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

export async function ensureMove(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.moves).where(eq(schema.moves.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchMove(pokeapiId);
  const typePokeapiId = client.extractIdFromUrl(data.type.url);
  const typeDbId = await ensureType(typePokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const vgMap = await buildIdMap(schema.versionGroups);
  const mapped = mappers.mapMove(data, typeDbId, genDbId, languageMap, langNameMap, vgMap);
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

export async function ensureItem(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.items).where(eq(schema.items.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchItem(pokeapiId);
  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const vgMap = await buildIdMap(schema.versionGroups);
  const mapped = mappers.mapItem(data, languageMap, langNameMap, vgMap);
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

export async function ensureEvolutionChain(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.evolutionChains).where(eq(schema.evolutionChains.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchEvolutionChain(pokeapiId);

  // Resolve baby trigger item if present
  let babyItemDbId: number | null = null;
  if (data.baby_trigger_item) {
    babyItemDbId = await ensureItem(client.extractIdFromUrl(data.baby_trigger_item.url));
  }

  // Insert chain first (without details)
  const result = await db.insert(schema.evolutionChains).values({
    pokeapiId,
    source: 'pokeapi',
    babyTriggerItemId: babyItemDbId,
  }).returning({ id: schema.evolutionChains.id });
  const chainDbId = result[0]!.id;

  // Ensure all species in the chain exist first
  const speciesIds = mappers.extractSpeciesFromChain(data.chain);
  const speciesDbMap = new Map<number, number>();
  for (const spId of speciesIds) {
    const dbId = await ensurePokemonSpeciesBase(spId, chainDbId);
    speciesDbMap.set(spId, dbId);
  }

  // Now insert evolution details
  const details = mappers.flattenEvolutionChain(data.chain);
  for (const detail of details) {
    const fromDbId = speciesDbMap.get(detail.fromSpeciesPokeapiId);
    const toDbId = speciesDbMap.get(detail.toSpeciesPokeapiId);
    if (!fromDbId || !toDbId) continue;

    let itemDbId: number | null = null;
    if (detail.itemPokeapiId) itemDbId = await ensureItem(detail.itemPokeapiId);

    let heldItemDbId: number | null = null;
    if (detail.heldItemPokeapiId) heldItemDbId = await ensureItem(detail.heldItemPokeapiId);

    let knownMoveDbId: number | null = null;
    if (detail.knownMovePokeapiId) knownMoveDbId = await ensureMove(detail.knownMovePokeapiId);

    let knownMoveTypeDbId: number | null = null;
    if (detail.knownMoveTypePokeapiId) knownMoveTypeDbId = await ensureType(detail.knownMoveTypePokeapiId);

    await db.insert(schema.evolutionDetails).values({
      chainId: chainDbId,
      fromSpeciesId: fromDbId,
      toSpeciesId: toDbId,
      trigger: detail.trigger,
      minLevel: detail.minLevel,
      minHappiness: detail.minHappiness,
      minBeauty: detail.minBeauty,
      minAffection: detail.minAffection,
      itemId: itemDbId,
      heldItemId: heldItemDbId,
      knownMoveId: knownMoveDbId,
      knownMoveTypeId: knownMoveTypeDbId,
      location: detail.location,
      timeOfDay: detail.timeOfDay,
      gender: detail.gender,
      relativePhysicalStats: detail.relativePhysicalStats,
      needsOverworldRain: detail.needsOverworldRain,
      turnUpsideDown: detail.turnUpsideDown,
    }).onConflictDoNothing();
  }

  await logImport('evolution_chain', pokeapiId, 'inserted');
  return chainDbId;
}

/**
 * Ensures a PokemonSpecies exists. This is the "base" version that inserts the
 * species record without requiring the full evolution chain to be resolved yet.
 * Used internally by ensureEvolutionChain to break circular deps.
 */
async function ensurePokemonSpeciesBase(pokeapiId: number, evolutionChainDbId: number | null): Promise<number> {
  const existing = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.pokeapiId, pokeapiId)).get();

  // Always fetch to upsert flavor texts, even if species already exists
  const data = await client.fetchPokemonSpecies(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  // Handle evolvesFrom — recursively ensure the pre-evolution
  let evolvesFromDbId: number | null = null;
  if (data.evolves_from_species) {
    const fromPokeapiId = client.extractIdFromUrl(data.evolves_from_species.url);
    evolvesFromDbId = await ensurePokemonSpeciesBase(fromPokeapiId, evolutionChainDbId);
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

  const mapped = mappers.mapPokemonSpecies(data, genDbId, evolutionChainDbId, evolvesFromDbId, languageMap, langNameMap, versionToVgMap);

  // If species already exists, only upsert flavor texts and return early
  if (existing) {
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
export async function ensurePokemonSpecies(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  // Fetch species to get evolution chain URL
  const data = await client.fetchPokemonSpecies(pokeapiId);
  const chainPokeapiId = client.extractIdFromUrl(data.evolution_chain.url);

  // Ensure evolution chain (which ensures all species in the chain)
  await ensureEvolutionChain(chainPokeapiId);

  // The species should exist now
  const species = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.pokeapiId, pokeapiId)).get();
  if (!species) throw new Error(`Failed to import species ${pokeapiId} via evolution chain`);
  return species.id;
}

export async function ensurePokemon(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.pokemon).where(eq(schema.pokemon.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchPokemon(pokeapiId);

  // Ensure species (and full evo chain)
  const speciesPokeapiId = client.extractIdFromUrl(data.species.url);
  const speciesDbId = await ensurePokemonSpecies(speciesPokeapiId);

  const mapped = mappers.mapPokemon(data, speciesDbId);
  const { id: pokemonDbId } = await protectedUpsert(schema.pokemon, mapped.main, pokeapiId);

  // Types
  for (const t of mapped.types) {
    const typeDbId = await ensureType(t.typePokeapiId);
    await db.insert(schema.pokemonTypes).values({ pokemonId: pokemonDbId, typeId: typeDbId, slot: t.slot }).onConflictDoNothing();
  }

  // Abilities
  for (const a of mapped.abilities) {
    const abilityDbId = await ensureAbility(a.abilityPokeapiId);
    await db.insert(schema.pokemonAbilities).values({ pokemonId: pokemonDbId, abilityId: abilityDbId, slot: a.slot, isHidden: a.isHidden }).onConflictDoNothing();
  }

  // Stats
  for (const s of mapped.stats) {
    await db.insert(schema.pokemonStats).values({ pokemonId: pokemonDbId, ...s }).onConflictDoNothing();
  }

  // Moves (with dependency resolution)
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

  // Forms
  for (const formRef of data.forms) {
    const formPokeapiId = client.extractIdFromUrl(formRef.url);
    await ensurePokemonForm(formPokeapiId, pokemonDbId, speciesDbId);
  }

  await logImport('pokemon', pokeapiId, 'inserted');
  return pokemonDbId;
}

async function ensurePokemonForm(pokeapiId: number, pokemonDbId: number, speciesDbId: number): Promise<number> {
  const existing = await db.select().from(schema.pokemonForms).where(eq(schema.pokemonForms.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchPokemonForm(pokeapiId);
  const mapped = mappers.mapPokemonForm(data, pokemonDbId, speciesDbId);
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
export async function importPokemonById(pokeapiId: number): Promise<number> {
  console.log(`🔴 Importing Pokémon #${pokeapiId}...`);
  const dbId = await ensurePokemon(pokeapiId);
  
  // Recursively import all other members of the same evolution chain
  const speciesRow = await db.select()
    .from(schema.pokemonSpecies)
    .innerJoin(schema.pokemon, eq(schema.pokemon.speciesId, schema.pokemonSpecies.id))
    .where(eq(schema.pokemon.id, dbId))
    .get();

  if (speciesRow && speciesRow.pokemon_species.evolutionChainId) {
    const chainEdges = await db.select()
      .from(schema.evolutionDetails)
      .where(eq(schema.evolutionDetails.chainId, speciesRow.pokemon_species.evolutionChainId));
      
    const speciesIds = new Set<number>();
    speciesIds.add(speciesRow.pokemon_species.id);
    for (const edge of chainEdges) {
      speciesIds.add(edge.fromSpeciesId);
      speciesIds.add(edge.toSpeciesId);
    }

    for (const spId of speciesIds) {
      if (spId !== speciesRow.pokemon_species.id) {
        const spRecord = await db.select().from(schema.pokemonSpecies).where(eq(schema.pokemonSpecies.id, spId)).get();
        if (spRecord) {
           const spData = await client.fetchPokemonSpecies(spRecord.pokeapiId);
           const defaultVariety = spData.varieties.find((v: any) => v.is_default);
           if (defaultVariety) {
             const defaultPokeapiId = client.extractIdFromUrl(defaultVariety.pokemon.url);
             if (defaultPokeapiId !== pokeapiId) {
                console.log(`  ↪ Also importing family member #${defaultPokeapiId}...`);
                await ensurePokemon(defaultPokeapiId);
             }
           }
        }
      }
    }
  }

  console.log(`✅ Pokémon #${pokeapiId} (and family) imported (db id: ${dbId})`);
  return dbId;
}

/** Import a range of Pokémon (e.g. 1-151 for Gen 1) */
export async function importPokemonByRange(from: number, to: number): Promise<ImportResult> {
  const result = emptyResult('pokemon');
  result.total = to - from + 1;

  for (let id = from; id <= to; id++) {
    try {
      await ensurePokemon(id);
      result.inserted++;
      console.log(`  ✅ #${id} done (${result.inserted}/${result.total})`);
    } catch (e) {
      result.errors.push({ id, error: (e as Error).message });
      console.error(`  ❌ #${id} failed: ${(e as Error).message}`);
    }
  }
  return result;
}
