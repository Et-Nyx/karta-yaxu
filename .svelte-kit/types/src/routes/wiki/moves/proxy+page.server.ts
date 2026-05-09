// @ts-nocheck
import { db } from '$lib/db';
import { eq, asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load = async () => {
  const movesList = await db.select({
    id: schema.moves.id,
    name: schema.moves.name,
    slug: schema.moves.slug,
    power: schema.moves.power,
    accuracy: schema.moves.accuracy,
    pp: schema.moves.pp,
    priority: schema.moves.priority,
    damageClass: schema.moves.damageClass,
    typeName: schema.types.name,
  })
  .from(schema.moves)
  .leftJoin(schema.types, eq(schema.moves.typeId, schema.types.id))
  .orderBy(asc(schema.moves.id));

  return { moves: movesList };
};
;null as any as PageServerLoad;