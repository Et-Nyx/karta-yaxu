import { db } from '$lib/db';
import { asc, eq } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const rows = await db.select({
    id:         schema.locations.id,
    name:       schema.locations.name,
    slug:       schema.locations.slug,
    regionName: schema.regions.name,
    regionSlug: schema.regions.slug,
  })
  .from(schema.locations)
  .leftJoin(schema.regions, eq(schema.locations.regionId, schema.regions.id))
  .orderBy(asc(schema.regions.name), asc(schema.locations.name));

  // Group by region in memory
  type LocationEntry = { id: number; name: string; slug: string };
  const regionMap = new Map<string, { name: string; slug: string; locations: LocationEntry[] }>();

  for (const row of rows) {
    const key = row.regionName ?? 'other';
    if (!regionMap.has(key)) {
      regionMap.set(key, { name: key, slug: row.regionSlug ?? 'other', locations: [] });
    }
    regionMap.get(key)!.locations.push({ id: row.id, name: row.name, slug: row.slug });
  }

  return {
    regions: [...regionMap.values()],
    total: rows.length,
  };
};
