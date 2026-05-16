// ============================================================================
// Import Locations — ensureLocation, ensureLocationArea, encounters
// ============================================================================

import { eq, and } from 'drizzle-orm';
import { db } from '$lib/db/index';
import * as schema from '$lib/db/schema/index';
import * as client from './client';
import * as mappers from './mappers';
import type { EncounterSlot } from './types';
import {
  protectedUpsert,
  logImport,
  buildIdMap,
  getLangNameMap,
  ensureRegion,
  ensureEncounterMethod,
  type ImportResult,
} from './import-service';

function emptyResult(entity: string): ImportResult {
  return { entity, total: 0, inserted: 0, updated: 0, skipped: 0, errors: [] };
}

// ── ensureLocation ─────────────────────────────────────────────────────────

export async function ensureLocation(
  pokeapiId: number,
): Promise<{ id: number; areas: { name: string; url: string }[] }> {
  const data = await client.fetchLocation(pokeapiId);

  const existing = await db
    .select()
    .from(schema.locations)
    .where(eq(schema.locations.pokeapiId, pokeapiId))
    .get();

  if (existing) return { id: existing.id, areas: data.areas };

  const regionDbId = data.region
    ? await ensureRegion(client.extractIdFromUrl(data.region.url))
    : null;

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapLocation(data, regionDbId, languageMap, langNameMap);

  const { action, id } = await protectedUpsert(schema.locations, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db
      .insert(schema.locationNames)
      .values({ locationId: id, ...n })
      .onConflictDoNothing();
  }
  await logImport('location', pokeapiId, action);
  return { id, areas: data.areas };
}

// ── ensureLocationArea ─────────────────────────────────────────────────────

export async function ensureLocationArea(
  pokeapiId: number,
  locationDbId: number,
): Promise<{ id: number; encounters: EncounterSlot[] }> {
  const data = await client.fetchLocationArea(pokeapiId);

  const existing = await db
    .select()
    .from(schema.locationAreas)
    .where(eq(schema.locationAreas.pokeapiId, pokeapiId))
    .get();

  if (existing) return { id: existing.id, encounters: data.pokemon_encounters };

  const languageMap = await buildIdMap(schema.languages);
  const langNameMap = await getLangNameMap();
  const mapped = mappers.mapLocationArea(data, locationDbId, languageMap, langNameMap);

  const { action, id } = await protectedUpsert(schema.locationAreas, mapped.main, pokeapiId);

  for (const n of mapped.names) {
    await db
      .insert(schema.locationAreaNames)
      .values({ locationAreaId: id, ...n })
      .onConflictDoNothing();
  }
  await logImport('location_area', pokeapiId, action);
  return { id, encounters: data.pokemon_encounters };
}

// ── importEncountersForArea ────────────────────────────────────────────────

export async function importEncountersForArea(
  areaId: number,
  slots: EncounterSlot[],
  pokemonMap: Map<string, number>,
  versionMap: Map<string, number>,
): Promise<void> {
  // Clear existing pokeapi encounters for this area (idempotency)
  await db
    .delete(schema.encounters)
    .where(
      and(
        eq(schema.encounters.locationAreaId, areaId),
        eq(schema.encounters.source, 'pokeapi'),
      ),
    );

  for (const slot of slots) {
    const pokemonDbId = pokemonMap.get(slot.pokemon.name);
    if (!pokemonDbId) continue; // pokemon not imported yet — skip silently

    for (const vd of slot.version_details) {
      const versionDbId = versionMap.get(vd.version.name);
      if (!versionDbId) continue; // version not imported yet — skip silently

      for (const detail of vd.encounter_details) {
        try {
          const methodDbId = await ensureEncounterMethod(
            client.extractIdFromUrl(detail.method.url),
          );
          await db.insert(schema.encounters).values({
            source: 'pokeapi',
            locationAreaId: areaId,
            pokemonId: pokemonDbId,
            versionId: versionDbId,
            encounterMethodId: methodDbId,
            conditionValueId: null,
            minLevel: detail.min_level,
            maxLevel: detail.max_level,
            chance: detail.chance,
            slot: null,
          });
        } catch {
          // skip encounter details with unknown methods
        }
      }
    }
  }
}

// ── importLocationsByRegion ────────────────────────────────────────────────

export async function importLocationsByRegion(regionPokeapiId: number): Promise<ImportResult> {
  const result = emptyResult('locations');

  const pokemonRows = await db
    .select({ id: schema.pokemon.id, slug: schema.pokemon.slug })
    .from(schema.pokemon);
  const pokemonMap = new Map<string, number>();
  for (const r of pokemonRows) pokemonMap.set(r.slug, r.id);

  const versionRows = await db
    .select({ id: schema.versions.id, slug: schema.versions.slug })
    .from(schema.versions);
  const versionMap = new Map<string, number>();
  for (const r of versionRows) versionMap.set(r.slug, r.id);

  const regionData = await client.fetchRegion(regionPokeapiId);
  result.total = regionData.locations.length;

  for (const locRef of regionData.locations) {
    const locPokeapiId = client.extractIdFromUrl(locRef.url);
    try {
      const { id: locDbId, areas } = await ensureLocation(locPokeapiId);
      for (const areaRef of areas) {
        const areaPokeapiId = client.extractIdFromUrl(areaRef.url);
        try {
          const { id: areaDbId, encounters } = await ensureLocationArea(areaPokeapiId, locDbId);
          await importEncountersForArea(areaDbId, encounters, pokemonMap, versionMap);
          result.inserted++;
        } catch (e) {
          result.errors.push({ id: areaPokeapiId, error: (e as Error).message });
        }
      }
    } catch (e) {
      result.errors.push({ id: locPokeapiId, error: (e as Error).message });
    }
  }

  return result;
}

// ── importAllLocations ─────────────────────────────────────────────────────

export async function importAllLocations(): Promise<ImportResult> {
  const result = emptyResult('locations');

  // Build lookup maps once to avoid per-area DB queries
  const pokemonRows = await db
    .select({ id: schema.pokemon.id, slug: schema.pokemon.slug })
    .from(schema.pokemon);
  const pokemonMap = new Map<string, number>();
  for (const r of pokemonRows) pokemonMap.set(r.slug, r.id);

  const versionRows = await db
    .select({ id: schema.versions.id, slug: schema.versions.slug })
    .from(schema.versions);
  const versionMap = new Map<string, number>();
  for (const r of versionRows) versionMap.set(r.slug, r.id);

  const locationList = await client.fetchList('location');
  result.total = locationList.results.length;

  for (const item of locationList.results) {
    const locPokeapiId = client.extractIdFromUrl(item.url);
    try {
      const { id: locDbId, areas } = await ensureLocation(locPokeapiId);

      for (const areaRef of areas) {
        const areaPokeapiId = client.extractIdFromUrl(areaRef.url);
        try {
          const { id: areaDbId, encounters } = await ensureLocationArea(areaPokeapiId, locDbId);
          await importEncountersForArea(areaDbId, encounters, pokemonMap, versionMap);
          result.inserted++;
        } catch (e) {
          result.errors.push({ id: areaPokeapiId, error: (e as Error).message });
        }
      }
    } catch (e) {
      result.errors.push({ id: locPokeapiId, error: (e as Error).message });
    }
  }

  return result;
}
