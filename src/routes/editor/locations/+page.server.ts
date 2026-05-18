import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq, sql } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { locations, locationAreas } from '$lib/db/schema/locations';
import { regions } from '$lib/db/schema/game-metadata';
import { recordEdit } from '$lib/editor/audit';
import { requireEditor } from '$lib/auth/guards';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const CreateSchema = z.object({
  name:     z.string().trim().min(1).max(100),
  slug:     z.string().trim().min(1).max(100).regex(slugRegex, 'slug: lowercase letters/numbers/hyphens only'),
  regionId: z.coerce.number().int().positive().optional().or(z.literal('').transform(() => undefined)),
});

export const load: PageServerLoad = async ({ locals }) => {
  requireEditor(locals);

  const rows = await db
    .select({
      id:         locations.id,
      slug:       locations.slug,
      name:       locations.name,
      source:     locations.source,
      regionId:   locations.regionId,
      regionName: regions.name,
      areaCount:  sql<number>`(select count(*) from location_areas where location_areas.location_id = ${locations.id})`,
    })
    .from(locations)
    .leftJoin(regions, eq(locations.regionId, regions.id))
    .orderBy(asc(locations.name));

  const [{ total }] = await db.select({ total: count() }).from(locations);

  const allRegions = await db
    .select({ id: regions.id, name: regions.name, slug: regions.slug })
    .from(regions)
    .orderBy(asc(regions.name));

  return { locations: rows, total, regions: allRegions };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    requireEditor(locals);
    const form = Object.fromEntries(await request.formData());
    const parsed = CreateSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, { createError: parsed.error.issues[0]?.message ?? 'Invalid', createValues: form });
    }

    const data = parsed.data;
    const [existing] = await db.select({ id: locations.id })
      .from(locations).where(eq(locations.slug, data.slug)).limit(1);
    if (existing) {
      return fail(400, { createError: 'slug already in use', createValues: form });
    }

    const [inserted] = await db.insert(locations).values({
      name:      data.name,
      slug:      data.slug,
      regionId:  data.regionId ?? null,
      source:    'custom',
      pokeapiId: null,
    }).returning({ id: locations.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'locations', inserted.id, null, data);
    }

    redirect(303, `/editor/locations/${data.slug}`);
  },
};
