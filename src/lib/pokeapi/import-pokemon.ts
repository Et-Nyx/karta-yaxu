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

  for (const edge of edges) {
    try {
      const fromPokemon = await defaultPokemonForSpeciesPokeapiId(edge.fromSpeciesPokeapiId);
      const toPokemon   = await defaultPokemonForSpeciesPokeapiId(edge.toSpeciesPokeapiId);
      if (!fromPokemon || !toPokemon) continue;

      // If we have already written standard evolution conditions for this target in this import run, skip subsequent ones
      if (processedTargets.has(toPokemon.id)) continue;

      // Set the parent pointer only if it's currently null (don't overwrite editor edits)
      const [child] = await db.select({
        parent: schema.pokemon.evolvesFromPokemonId,
      }).from(schema.pokemon).where(eq(schema.pokemon.id, toPokemon.id)).limit(1);
      if (child?.parent == null) {
        await db.update(schema.pokemon)
          .set({ evolvesFromPokemonId: fromPokemon.id })
          .where(eq(schema.pokemon.id, toPokemon.id));
      }

      // Resolve item/move/type FKs
      let itemDbId:          number | null = null;
      if (edge.itemPokeapiId)          itemDbId          = await ensureItem(edge.itemPokeapiId);
      let heldItemDbId:      number | null = null;
      if (edge.heldItemPokeapiId)      heldItemDbId      = await ensureItem(edge.heldItemPokeapiId);
      let knownMoveDbId:     number | null = null;
      if (edge.knownMovePokeapiId)     knownMoveDbId     = await ensureMove(edge.knownMovePokeapiId);
      let knownMoveTypeDbId: number | null = null;
      if (edge.knownMoveTypePokeapiId) knownMoveTypeDbId = await ensureType(edge.knownMoveTypePokeapiId);

      // Source-protected upsert of conditions row
      const [existingCond] = await db.select({ source: schema.pokemonEvolutionDetails.source })
        .from(schema.pokemonEvolutionDetails)
        .where(eq(schema.pokemonEvolutionDetails.pokemonId, toPokemon.id)).limit(1);

      if (!existingCond || existingCond.source === 'pokeapi') {
        const payload = {
          pokemonId:       toPokemon.id,
          trigger:         edge.trigger,
          minLevel:        edge.minLevel ?? null,
          itemId:          itemDbId,
          heldItemId:      heldItemDbId,
          knownMoveId:     knownMoveDbId,
          knownMoveTypeId: knownMoveTypeDbId,
          minHappiness:    edge.minHappiness ?? null,
          timeOfDay:       edge.timeOfDay ?? null,
          gender:          edge.gender ?? null,
          location:        edge.location ?? null,
          source:          'pokeapi' as const,
        };
        await db.insert(schema.pokemonEvolutionDetails).values(payload).onConflictDoUpdate({
          target: schema.pokemonEvolutionDetails.pokemonId,
          set: {
            trigger:         payload.trigger,
            minLevel:        payload.minLevel,
            itemId:          payload.itemId,
            heldItemId:      payload.heldItemId,
            knownMoveId:     payload.knownMoveId,
            knownMoveTypeId: payload.knownMoveTypeId,
            minHappiness:    payload.minHappiness,
            timeOfDay:       payload.timeOfDay,
            gender:          payload.gender,
            location:        payload.location,
            source:          'pokeapi',
          },
        });
        processedTargets.add(toPokemon.id);
      } else {
        // Even if existing source is 'edited' or 'custom', we mark it as processed so we don't add alternate PokeAPI edges
        processedTargets.add(toPokemon.id);
      }
    } catch {
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

    // Import each family member's default pokemon
    for (const spPokeapiId of speciesPokeapiIds) {
      if (spPokeapiId === speciesPokeapiId) continue; // self
      try {
        const spData = await client.fetchPokemonSpecies(spPokeapiId);
        const defaultVariety = spData.varieties.find((v: any) => v.is_default);
        if (defaultVariety) {
          const defaultPokeapiId = client.extractIdFromUrl(defaultVariety.pokemon.url);
          if (defaultPokeapiId !== pokeapiId) {
            console.log(`  ↪ Also importing family member #${defaultPokeapiId}...`);
            await ensurePokemon(defaultPokeapiId, opts);
          }
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

/**
 * For every species already in the DB, imports all non-default varieties (regional forms).
 * Skips cosmetic/event-only variants (totems, cap Pikachus, Let's Go starters).
 * Call this after a range import to pick up Alolan, Galarian, Hisuian, Gigantamax, etc.
 */
export async function importAllRegionalForms(
  opts: { forceRefresh?: boolean; overwriteEdited?: boolean } = {},
): Promise<ImportResult> {
  const result = emptyResult('regional_forms');

  const allSpecies = await db.select({
    id: schema.pokemonSpecies.id,
    pokeapiId: schema.pokemonSpecies.pokeapiId,
    name: schema.pokemonSpecies.name,
  }).from(schema.pokemonSpecies);

  for (const species of allSpecies) {
    if (!species.pokeapiId) continue;
    try {
      const data = await client.fetchPokemonSpecies(species.pokeapiId);
      const varieties = data.varieties.filter(
        (v: any) => !v.is_default && !isUnwantedVariety(v.pokemon.name)
      );
      result.total += varieties.length;

      for (const variety of varieties) {
        const pokemonPokeapiId = client.extractIdFromUrl(variety.pokemon.url);
        try {
          console.log(`  🌍 Importing variant: ${variety.pokemon.name} (#${pokemonPokeapiId})`);
          await ensurePokemon(pokemonPokeapiId, opts);
          result.inserted++;
        } catch (e) {
          result.errors.push({ id: pokemonPokeapiId, error: (e as Error).message });
        }
      }
    } catch (e) {
      result.errors.push({ id: species.pokeapiId, error: (e as Error).message });
    }
  }

  console.log(`✅ importAllRegionalForms: ${result.inserted}/${result.total} imported`);
  return result;
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
