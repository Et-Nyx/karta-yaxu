import { db } from '$lib/db';
import { asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const abilitiesList = await db.select()
    .from(schema.abilities)
    .orderBy(asc(schema.abilities.id));

  return { abilities: abilitiesList };
};
