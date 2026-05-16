import { db } from '$lib/db';
import { eq, asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const pokedexes = await db.select({
    id:           schema.pokedexes.id,
    name:         schema.pokedexes.name,
    slug:         schema.pokedexes.slug,
    isMainSeries: schema.pokedexes.isMainSeries,
    regionName:   schema.regions.name,
    regionSlug:   schema.regions.slug,
  })
  .from(schema.pokedexes)
  .leftJoin(schema.regions, eq(schema.pokedexes.regionId, schema.regions.id))
  .orderBy(asc(schema.pokedexes.id));

  return { pokedexes, total: pokedexes.length };
};
