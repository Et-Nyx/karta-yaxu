import { db } from '$lib/db';
import { asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const items = await db.select({
    id: schema.items.id,
    name: schema.items.name,
    slug: schema.items.slug,
    category: schema.items.category,
    cost: schema.items.cost,
    spriteUrl: schema.items.spriteUrl,
  })
  .from(schema.items)
  .orderBy(asc(schema.items.name));

  return { items };
};
