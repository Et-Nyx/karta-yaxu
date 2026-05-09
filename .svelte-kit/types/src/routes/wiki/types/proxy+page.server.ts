// @ts-nocheck
import { db } from '$lib/db';
import { asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load = async () => {
  const typesList = await db.select({
    id: schema.types.id,
    name: schema.types.name,
    slug: schema.types.slug,
    iconUrl: schema.types.iconUrl,
  })
  .from(schema.types)
  .orderBy(asc(schema.types.id));

  return { types: typesList };
};
;null as any as PageServerLoad;