// ============================================================================
// Import Service — Core utilities and game metadata imports
// ============================================================================

import { eq } from 'drizzle-orm';
import { db } from '$lib/db/index';
import * as schema from '$lib/db/schema/index';
import * as client from './client';
import * as mappers from './mappers';
import type { IdMap } from './mappers';
import { importEvents } from './import-events';

// ── Types ──────────────────────────────────────────────────────────────────

export type UpsertAction = 'inserted' | 'updated' | 'skipped';

export interface ImportResult {
  entity: string;
  total: number;
  inserted: number;
  updated: number;
  skipped: number;
  errors: Array<{ id: number; error: string }>;
}

function emptyResult(entity: string): ImportResult {
  return { entity, total: 0, inserted: 0, updated: 0, skipped: 0, errors: [] };
}

// ── Batch Processing ───────────────────────────────────────────────────────

export async function batchProcess<T>(
  items: T[],
  fn: (item: T) => Promise<void>,
  concurrency: number = 5,
): Promise<void> {
  for (let i = 0; i < items.length; i += concurrency) {
    const batch = items.slice(i, i + concurrency);
    await Promise.all(batch.map(fn));
  }
}

// ── Import Log ─────────────────────────────────────────────────────────────

export async function logImport(entityType: string, pokeapiId: number, action: 'inserted' | 'updated' | 'skipped', reason?: string) {
  const timestamp = new Date().toISOString();
  
  // Fire event for SSE listeners
  importEvents.emit('log', {
    entityType,
    pokeapiId,
    action,
    reason,
    timestamp
  });

  await db.insert(schema.importLog).values({ entityType, pokeapiId, action, reason: reason ?? null });
}

// ── Language Name Map Cache ────────────────────────────────────────────────
// Maps PokeAPI language name (e.g. "en") → pokeapi_id (e.g. 9)

let _langNameToPokeapiId: Map<string, number> | null = null;

export async function getLangNameMap(): Promise<Map<string, number>> {
  if (_langNameToPokeapiId) return _langNameToPokeapiId;
  const langs = await db.select({ name: schema.languages.name, pokeapiId: schema.languages.pokeapiId }).from(schema.languages);
  _langNameToPokeapiId = new Map();
  for (const l of langs) {
    if (l.pokeapiId !== null) _langNameToPokeapiId.set(l.name, l.pokeapiId);
  }
  return _langNameToPokeapiId;
}

export function clearLangNameCache() { _langNameToPokeapiId = null; }

// ── Generic ID Map Builder ─────────────────────────────────────────────────

export async function buildIdMap(table: any): Promise<IdMap> {
  const rows = await db.select({ id: table.id, pokeapiId: table.pokeapiId }).from(table);
  const map: IdMap = new Map();
  for (const r of rows) {
    if (r.pokeapiId !== null) map.set(r.pokeapiId, r.id);
  }
  return map;
}

// ── Protected Upsert ───────────────────────────────────────────────────────

export async function protectedUpsert(
  table: any,
  data: Record<string, unknown>,
  pokeapiId: number,
): Promise<{ action: UpsertAction; id: number }> {
  const existing = await db.select().from(table).where(eq(table.pokeapiId, pokeapiId)).get();

  if (existing) {
    if (existing.source && existing.source !== 'pokeapi') {
      return { action: 'skipped', id: existing.id };
    }
    await db.update(table).set(data).where(eq(table.id, existing.id));
    return { action: 'updated', id: existing.id };
  }

  const result = await db.insert(table).values(data).returning({ id: table.id });
  return { action: 'inserted', id: result[0]!.id };
}

// ── Ensure Functions (resolve-or-import) ───────────────────────────────────

export async function ensureLanguage(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.languages).where(eq(schema.languages.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchLanguage(pokeapiId);
  const mapped = mappers.mapLanguage(data);
  const { id } = await protectedUpsert(schema.languages, mapped.main, pokeapiId);
  await logImport('language', pokeapiId, 'inserted');
  clearLangNameCache();
  return id;
}

export async function ensureRegion(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.regions).where(eq(schema.regions.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const data = await client.fetchRegion(pokeapiId);
  const mapped = mappers.mapRegion(data, languageMap, langNameMap);
  const { id } = await protectedUpsert(schema.regions, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.regionNames).values({ regionId: id, ...n }).onConflictDoNothing();
  }
  await logImport('region', pokeapiId, 'inserted');
  return id;
}

export async function ensureGeneration(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.generations).where(eq(schema.generations.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchGeneration(pokeapiId);
  const regionPokeapiId = client.extractIdFromUrl(data.main_region.url);
  const regionDbId = await ensureRegion(regionPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapGeneration(data, regionDbId, languageMap, langNameMap);
  const { id } = await protectedUpsert(schema.generations, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.generationNames).values({ generationId: id, ...n }).onConflictDoNothing();
  }
  await logImport('generation', pokeapiId, 'inserted');
  return id;
}

export async function ensureVersionGroup(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.versionGroups).where(eq(schema.versionGroups.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchVersionGroup(pokeapiId);
  const genPokeapiId = client.extractIdFromUrl(data.generation.url);
  const genDbId = await ensureGeneration(genPokeapiId);

  const mapped = mappers.mapVersionGroup(data, genDbId);
  const { id } = await protectedUpsert(schema.versionGroups, mapped.main, pokeapiId);
  await logImport('version_group', pokeapiId, 'inserted');
  return id;
}

export async function ensureVersion(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.versions).where(eq(schema.versions.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchVersion(pokeapiId);
  const vgPokeapiId = client.extractIdFromUrl(data.version_group.url);
  const vgDbId = await ensureVersionGroup(vgPokeapiId);

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapVersion(data, vgDbId, languageMap, langNameMap);
  const { id } = await protectedUpsert(schema.versions, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db.insert(schema.versionNames).values({ versionId: id, ...n }).onConflictDoNothing();
  }
  await logImport('version', pokeapiId, 'inserted');
  return id;
}

export async function ensureNature(
  pokeapiId: number,
  opts: { forceRefresh?: boolean } = {},
): Promise<number> {
  const existing = await db.select().from(schema.natures).where(eq(schema.natures.pokeapiId, pokeapiId)).get();
  if (existing && !opts.forceRefresh) return existing.id;

  const data = await client.fetchNature(pokeapiId);
  const mapped = mappers.mapNature(data);
  const { id } = await protectedUpsert(schema.natures, mapped.main, pokeapiId);
  await logImport('nature', pokeapiId, 'inserted');
  return id;
}

export async function ensureEggGroup(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.eggGroups).where(eq(schema.eggGroups.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchEggGroup(pokeapiId);
  const mapped = mappers.mapEggGroup(data);
  const result = await db.insert(schema.eggGroups).values(mapped.main).returning({ id: schema.eggGroups.id });
  await logImport('egg_group', pokeapiId, 'inserted');
  return result[0]!.id;
}

export async function ensureMoveLearnMethod(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.moveLearnMethods).where(eq(schema.moveLearnMethods.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchMoveLearnMethod(pokeapiId);
  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapMoveLearnMethod(data, languageMap, langNameMap);
  const result = await db.insert(schema.moveLearnMethods).values(mapped.main).returning({ id: schema.moveLearnMethods.id });
  const id = result[0]!.id;

  for (const n of mapped.names) {
    await db.insert(schema.moveLearnMethodNames).values({ moveLearnMethodId: id, ...n }).onConflictDoNothing();
  }
  await logImport('move_learn_method', pokeapiId, 'inserted');
  return id;
}

export async function ensureEncounterMethod(pokeapiId: number): Promise<number> {
  const existing = await db.select().from(schema.encounterMethods).where(eq(schema.encounterMethods.pokeapiId, pokeapiId)).get();
  if (existing) return existing.id;

  const data = await client.fetchEncounterMethod(pokeapiId);
  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapEncounterMethod(data, languageMap, langNameMap);
  const result = await db.insert(schema.encounterMethods).values(mapped.main).returning({ id: schema.encounterMethods.id });
  const id = result[0]!.id;

  for (const n of mapped.names) {
    await db.insert(schema.encounterMethodNames).values({ encounterMethodId: id, ...n }).onConflictDoNothing();
  }
  await logImport('encounter_method', pokeapiId, 'inserted');
  return id;
}

// ── Batch Imports (Game Metadata) ──────────────────────────────────────────

export async function importAllLanguages(): Promise<ImportResult> {
  const result = emptyResult('languages');
  const list = await client.fetchList('language');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      const pokeapiId = client.extractIdFromUrl(item.url);
      await ensureLanguage(pokeapiId);
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  clearLangNameCache();
  return result;
}

export async function importAllRegions(): Promise<ImportResult> {
  const result = emptyResult('regions');
  const list = await client.fetchList('region');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      const pokeapiId = client.extractIdFromUrl(item.url);
      await ensureRegion(pokeapiId);
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllGenerations(): Promise<ImportResult> {
  const result = emptyResult('generations');
  const list = await client.fetchList('generation');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureGeneration(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllVersionGroups(): Promise<ImportResult> {
  const result = emptyResult('version_groups');
  const list = await client.fetchList('version-group');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureVersionGroup(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllVersions(): Promise<ImportResult> {
  const result = emptyResult('versions');
  const list = await client.fetchList('version');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureVersion(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllNatures(): Promise<ImportResult> {
  const result = emptyResult('natures');
  const list = await client.fetchList('nature');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureNature(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllEggGroups(): Promise<ImportResult> {
  const result = emptyResult('egg_groups');
  const list = await client.fetchList('egg-group');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureEggGroup(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllMoveLearnMethods(): Promise<ImportResult> {
  const result = emptyResult('move_learn_methods');
  const list = await client.fetchList('move-learn-method');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureMoveLearnMethod(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllEncounterMethods(): Promise<ImportResult> {
  const result = emptyResult('encounter_methods');
  const list = await client.fetchList('encounter-method');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensureEncounterMethod(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function ensurePokedex(pokeapiId: number): Promise<number> {
  const data = await client.fetchPokedex(pokeapiId);

  let regionDbId: number | null = null;
  if (data.region) {
    regionDbId = await ensureRegion(client.extractIdFromUrl(data.region.url));
  }

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapPokedex(data, regionDbId, languageMap, langNameMap);
  const { id, action } = await protectedUpsert(schema.pokedexes, mapped.main, pokeapiId);

  if (action === 'inserted') {
    for (const n of mapped.names) {
      await db.insert(schema.pokedexNames).values({ pokedexId: id, ...n }).onConflictDoNothing();
    }
    await logImport('pokedex', pokeapiId, 'inserted');
  }

  // Always sync entries so re-running picks up newly imported species
  const speciesMap = await buildIdMap(schema.pokemonSpecies);
  for (const entry of data.pokemon_entries) {
    const speciesDbId = speciesMap.get(client.extractIdFromUrl(entry.pokemon_species.url));
    if (!speciesDbId) continue;
    await db.insert(schema.pokedexEntries)
      .values({ pokedexId: id, speciesId: speciesDbId, entryNumber: entry.entry_number })
      .onConflictDoUpdate({
        target: [schema.pokedexEntries.pokedexId, schema.pokedexEntries.speciesId],
        set: { entryNumber: entry.entry_number },
      });
  }

  return id;
}

export async function importPokedexesByVersionGroup(vgPokeapiId: number): Promise<ImportResult> {
  const result = emptyResult('pokedexes');
  const vgData = await client.fetchVersionGroup(vgPokeapiId);
  result.total = vgData.pokedexes.length;
  for (const p of vgData.pokedexes) {
    try {
      await ensurePokedex(client.extractIdFromUrl(p.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(p.url), error: (e as Error).message });
    }
  }
  return result;
}

export async function importAllPokedexes(): Promise<ImportResult> {
  const result = emptyResult('pokedexes');
  const list = await client.fetchList('pokedex');
  result.total = list.results.length;

  for (const item of list.results) {
    try {
      await ensurePokedex(client.extractIdFromUrl(item.url));
      result.inserted++;
    } catch (e) {
      result.errors.push({ id: client.extractIdFromUrl(item.url), error: (e as Error).message });
    }
  }
  return result;
}

/** Import all game metadata in dependency order */
export async function importAllGameData(): Promise<ImportResult[]> {
  const results: ImportResult[] = [];
  console.log('📦 Importing all game data...');

  console.log('  → Languages...');
  results.push(await importAllLanguages());
  console.log('  → Regions...');
  results.push(await importAllRegions());
  console.log('  → Generations...');
  results.push(await importAllGenerations());
  console.log('  → Version Groups...');
  results.push(await importAllVersionGroups());
  console.log('  → Versions...');
  results.push(await importAllVersions());
  console.log('  → Natures...');
  results.push(await importAllNatures());
  console.log('  → Egg Groups...');
  results.push(await importAllEggGroups());
  console.log('  → Move Learn Methods...');
  results.push(await importAllMoveLearnMethods());
  console.log('  → Encounter Methods...');
  results.push(await importAllEncounterMethods());

  console.log('✅ Game data import complete.');
  return results;
}
