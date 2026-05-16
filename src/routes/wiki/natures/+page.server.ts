import { db } from '$lib/db';
import { asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const natures = await db
    .select()
    .from(schema.natures)
    .orderBy(asc(schema.natures.name));

  return { natures };
};
