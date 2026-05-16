import { db } from '$lib/db';
import { eq, and, asc, desc, inArray } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, url }) => {
  const { slug } = params;

  // 1. Location
  const locList = await db.select({
    id:         schema.locations.id,
    name:       schema.locations.name,
    slug:       schema.locations.slug,
    regionName: schema.regions.name,
    regionSlug: schema.regions.slug,
  })
  .from(schema.locations)
  .leftJoin(schema.regions, eq(schema.locations.regionId, schema.regions.id))
  .where(eq(schema.locations.slug, slug))
  .limit(1);

  if (locList.length === 0) throw error(404, 'Location not found');
  const location = locList[0];

  // 2. Location areas
  const areas = await db.select()
    .from(schema.locationAreas)
    .where(eq(schema.locationAreas.locationId, location.id))
    .orderBy(asc(schema.locationAreas.gameIndex));

  if (areas.length === 0) {
    return { location, areas: [], availableVersions: [], selectedVersionId: null };
  }

  const areaIds = areas.map(a => a.id);

  // 3. Distinct versions that have encounters in these areas
  const versionRows = await db.selectDistinct({
    id:   schema.versions.id,
    name: schema.versions.name,
    slug: schema.versions.slug,
  })
  .from(schema.encounters)
  .innerJoin(schema.versions, eq(schema.encounters.versionId, schema.versions.id))
  .where(inArray(schema.encounters.locationAreaId, areaIds))
  .orderBy(asc(schema.versions.id));

  const availableVersions = versionRows;

  // 4. Selected version (from query param or first available)
  const versionParam = url.searchParams.get('version');
  let selectedVersion = versionParam
    ? availableVersions.find(v => v.slug === versionParam || v.id === Number(versionParam))
    : null;
  if (!selectedVersion && availableVersions.length > 0) {
    const frlg = availableVersions.find(v => v.slug === 'firered' || v.slug === 'leafgreen');
    selectedVersion = frlg ?? availableVersions[0];
  }

  const selectedVersionId = selectedVersion?.id ?? null;

  // 5. Encounters for each area, filtered by selected version
  type EncounterRow = {
    areaId: number;
    pokemonName: string;
    pokemonSlug: string;
    pokemonSprite: string | null;
    methodName: string;
    minLevel: number;
    maxLevel: number;
    chance: number;
  };

  let encounterRows: EncounterRow[] = [];

  if (selectedVersionId !== null) {
    encounterRows = await db.select({
      areaId:        schema.encounters.locationAreaId,
      pokemonName:   schema.pokemon.name,
      pokemonSlug:   schema.pokemon.slug,
      pokemonSprite: schema.pokemon.spriteDefault,
      methodName:    schema.encounterMethods.name,
      minLevel:      schema.encounters.minLevel,
      maxLevel:      schema.encounters.maxLevel,
      chance:        schema.encounters.chance,
    })
    .from(schema.encounters)
    .innerJoin(schema.pokemon, eq(schema.encounters.pokemonId, schema.pokemon.id))
    .innerJoin(schema.encounterMethods, eq(schema.encounters.encounterMethodId, schema.encounterMethods.id))
    .where(and(
      inArray(schema.encounters.locationAreaId, areaIds),
      eq(schema.encounters.versionId, selectedVersionId),
    ))
    .orderBy(desc(schema.encounters.chance), asc(schema.pokemon.name));
  }

  // 6. Group encounters by area
  const encountersByArea = new Map<number, EncounterRow[]>();
  for (const row of encounterRows) {
    if (!encountersByArea.has(row.areaId)) encountersByArea.set(row.areaId, []);
    encountersByArea.get(row.areaId)!.push(row);
  }

  const areasWithEncounters = areas.map(area => ({
    ...area,
    encounters: encountersByArea.get(area.id) ?? [],
  }));

  return {
    location,
    areas: areasWithEncounters,
    availableVersions,
    selectedVersionId,
  };
};
