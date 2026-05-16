import { db } from '$lib/db';
import { eq, asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  const { slug } = params;

  const vgList = await db.select({
    id:           schema.versionGroups.id,
    name:         schema.versionGroups.name,
    slug:         schema.versionGroups.slug,
    generationId: schema.versionGroups.generationId,
    order:        schema.versionGroups.order,
  })
  .from(schema.versionGroups)
  .where(eq(schema.versionGroups.slug, slug))
  .limit(1);

  if (vgList.length === 0) throw error(404, 'Version group not found');
  const versionGroup = vgList[0];

  const versions = await db.select({
    id:   schema.versions.id,
    name: schema.versions.name,
    slug: schema.versions.slug,
  })
  .from(schema.versions)
  .where(eq(schema.versions.versionGroupId, versionGroup.id))
  .orderBy(asc(schema.versions.id));

  const genList = await db.select({
    id:    schema.generations.id,
    name:  schema.generations.name,
    slug:  schema.generations.slug,
    order: schema.generations.order,
  })
  .from(schema.generations)
  .where(eq(schema.generations.id, versionGroup.generationId))
  .limit(1);

  const generation = genList[0] ?? null;

  return { versionGroup, versions, generation };
};
