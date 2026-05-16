import { db } from '$lib/db';
import { asc } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const gens = await db.select({
    id:    schema.generations.id,
    name:  schema.generations.name,
    slug:  schema.generations.slug,
    order: schema.generations.order,
  })
  .from(schema.generations)
  .orderBy(asc(schema.generations.order));

  const vgs = await db.select({
    id:           schema.versionGroups.id,
    name:         schema.versionGroups.name,
    slug:         schema.versionGroups.slug,
    generationId: schema.versionGroups.generationId,
    order:        schema.versionGroups.order,
  })
  .from(schema.versionGroups)
  .orderBy(asc(schema.versionGroups.order));

  const vs = await db.select({
    id:             schema.versions.id,
    name:           schema.versions.name,
    slug:           schema.versions.slug,
    versionGroupId: schema.versions.versionGroupId,
  })
  .from(schema.versions)
  .orderBy(asc(schema.versions.id));

  // Group versions by versionGroupId
  const versionsByGroup = new Map<number, { id: number; name: string; slug: string }[]>();
  for (const v of vs) {
    if (!versionsByGroup.has(v.versionGroupId)) versionsByGroup.set(v.versionGroupId, []);
    versionsByGroup.get(v.versionGroupId)!.push({ id: v.id, name: v.name, slug: v.slug });
  }

  // Group version groups by generationId
  type VGEntry = { id: number; name: string; slug: string; versions: { id: number; name: string; slug: string }[] };
  const vgsByGen = new Map<number, VGEntry[]>();
  for (const vg of vgs) {
    if (!vgsByGen.has(vg.generationId)) vgsByGen.set(vg.generationId, []);
    vgsByGen.get(vg.generationId)!.push({
      id:       vg.id,
      name:     vg.name,
      slug:     vg.slug,
      versions: versionsByGroup.get(vg.id) ?? [],
    });
  }

  const generations = gens
    .map(gen => ({ ...gen, versionGroups: vgsByGen.get(gen.id) ?? [] }))
    .filter(gen => gen.versionGroups.length > 0);

  return { generations };
};
