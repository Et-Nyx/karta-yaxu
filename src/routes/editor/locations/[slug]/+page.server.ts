import { error, fail, redirect } from '@sveltejs/kit';
import { asc, count, desc, eq, inArray, and } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { locations, locationAreas, encounterMethods, encounterConditionValues } from '$lib/db/schema/locations';
import { regions, versions } from '$lib/db/schema/game-metadata';
import { encounters, pokemon } from '$lib/db/schema/pokemon';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { requireEditor } from '$lib/auth/guards';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const UpdateSchema = z.object({
  name:     z.string().trim().min(1).max(100),
  slug:     z.string().trim().min(1).max(100).regex(slugRegex, 'slug: lowercase letters/numbers/hyphens only'),
  regionId: z.coerce.number().int().positive().optional().or(z.literal('').transform(() => undefined)),
});

const AreaSchema = z.object({
  name:      z.string().trim().min(1).max(100),
  slug:      z.string().trim().min(1).max(100).regex(slugRegex, 'slug: lowercase letters/numbers/hyphens only'),
  gameIndex: z.coerce.number().int().min(0).optional().or(z.literal('').transform(() => undefined)),
});

const EncounterSchema = z.object({
  locationAreaId:    z.coerce.number().int().positive(),
  pokemonId:         z.coerce.number().int().positive(),
  versionId:         z.coerce.number().int().positive(),
  encounterMethodId: z.coerce.number().int().positive(),
  conditionValueId:  z.coerce.number().int().positive().optional().or(z.literal('').transform(() => undefined)),
  minLevel:          z.coerce.number().int().min(1).max(100),
  maxLevel:          z.coerce.number().int().min(1).max(100),
  chance:            z.coerce.number().int().min(1).max(100),
  slot:              z.coerce.number().int().min(0).optional().or(z.literal('').transform(() => undefined)),
});

export const load: PageServerLoad = async ({ params, locals }) => {
  requireEditor(locals);

  const [loc] = await db
    .select({
      id:         locations.id,
      slug:       locations.slug,
      name:       locations.name,
      source:     locations.source,
      regionId:   locations.regionId,
      regionName: regions.name,
    })
    .from(locations)
    .leftJoin(regions, eq(locations.regionId, regions.id))
    .where(eq(locations.slug, params.slug))
    .limit(1);

  if (!loc) error(404, 'Location not found');

  const areas = await db
    .select()
    .from(locationAreas)
    .where(eq(locationAreas.locationId, loc.id))
    .orderBy(asc(locationAreas.gameIndex), asc(locationAreas.name));

  const allRegions = await db
    .select({ id: regions.id, name: regions.name })
    .from(regions)
    .orderBy(asc(regions.name));

  const areaIds = areas.map(a => a.id);

  const locationEncounters = areaIds.length === 0 ? [] : await db
    .select({
      id:                 encounters.id,
      source:             encounters.source,
      locationAreaId:     encounters.locationAreaId,
      areaName:           locationAreas.name,
      pokemonId:          encounters.pokemonId,
      pokemonName:        pokemon.name,
      pokemonSlug:        pokemon.slug,
      versionId:          encounters.versionId,
      versionName:        versions.name,
      encounterMethodId:  encounters.encounterMethodId,
      methodName:         encounterMethods.name,
      conditionValueId:   encounters.conditionValueId,
      conditionValueName: encounterConditionValues.name,
      minLevel:           encounters.minLevel,
      maxLevel:           encounters.maxLevel,
      chance:             encounters.chance,
      slot:               encounters.slot,
    })
    .from(encounters)
    .innerJoin(locationAreas, eq(encounters.locationAreaId, locationAreas.id))
    .innerJoin(pokemon, eq(encounters.pokemonId, pokemon.id))
    .innerJoin(versions, eq(encounters.versionId, versions.id))
    .innerJoin(encounterMethods, eq(encounters.encounterMethodId, encounterMethods.id))
    .leftJoin(encounterConditionValues, eq(encounters.conditionValueId, encounterConditionValues.id))
    .where(inArray(encounters.locationAreaId, areaIds))
    .orderBy(asc(versions.id), asc(locationAreas.gameIndex), asc(encounterMethods.order), desc(encounters.chance), asc(pokemon.name));

  const allPokemon = await db
    .select({ id: pokemon.id, name: pokemon.name })
    .from(pokemon)
    .orderBy(asc(pokemon.name));

  const allVersions = await db
    .select({ id: versions.id, name: versions.name })
    .from(versions)
    .orderBy(asc(versions.id));

  const allMethods = await db
    .select({ id: encounterMethods.id, name: encounterMethods.name })
    .from(encounterMethods)
    .orderBy(asc(encounterMethods.order));

  const allConditionValues = await db
    .select({ id: encounterConditionValues.id, name: encounterConditionValues.name })
    .from(encounterConditionValues)
    .orderBy(asc(encounterConditionValues.name));

  return {
    location: loc,
    areas,
    regions: allRegions,
    encounters: locationEncounters,
    allPokemon,
    allVersions,
    allMethods,
    allConditionValues,
  };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    requireEditor(locals);
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, { updateError: parsed.error.issues[0]?.message ?? 'Invalid', values: form });
    }

    const data = parsed.data;
    const [current] = await db.select().from(locations).where(eq(locations.slug, params.slug)).limit(1);
    if (!current) error(404, 'Location not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: locations.id })
        .from(locations).where(eq(locations.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug already in use', values: form });
    }

    await db.update(locations).set({
      name:     data.name,
      slug:     data.slug,
      regionId: data.regionId ?? null,
      source:   bumpSource(current.source),
    }).where(eq(locations.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'locations', current.id, current, data);
    }

    if (data.slug !== params.slug) redirect(303, `/editor/locations/${data.slug}`);
    return { updateSuccess: true };
  },

  createArea: async ({ request, params, locals }) => {
    requireEditor(locals);
    const form = Object.fromEntries(await request.formData());
    const parsed = AreaSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, { areaError: parsed.error.issues[0]?.message ?? 'Invalid', areaValues: form });
    }

    const data = parsed.data;
    const [loc] = await db.select({ id: locations.id })
      .from(locations).where(eq(locations.slug, params.slug)).limit(1);
    if (!loc) error(404, 'Location not found');

    const [clash] = await db.select({ id: locationAreas.id })
      .from(locationAreas).where(eq(locationAreas.slug, data.slug)).limit(1);
    if (clash) return fail(400, { areaError: 'Area slug already in use', areaValues: form });

    await db.insert(locationAreas).values({
      name:       data.name,
      slug:       data.slug,
      locationId: loc.id,
      gameIndex:  data.gameIndex ?? null,
      source:     'custom',
      pokeapiId:  null,
    });

    return { areaSuccess: true };
  },

  deleteArea: async ({ request, locals }) => {
    requireEditor(locals);
    const id = Number((await request.formData()).get('id'));
    if (!id) return fail(400, { error: 'Invalid' });

    const [area] = await db.select({ source: locationAreas.source })
      .from(locationAreas).where(eq(locationAreas.id, id)).limit(1);
    if (!area) return fail(404, { error: 'Area not found' });
    if (area.source !== 'custom') return fail(400, { error: 'Cannot delete PokeAPI area' });

    await db.delete(locationAreas).where(eq(locationAreas.id, id));
    return {};
  },

  createEncounter: async ({ request, params, locals }) => {
    requireEditor(locals);
    const form = Object.fromEntries(await request.formData());
    const parsed = EncounterSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, { encError: parsed.error.issues[0]?.message ?? 'Invalid', encValues: form });
    }

    const data = parsed.data;

    const [loc] = await db.select({ id: locations.id })
      .from(locations).where(eq(locations.slug, params.slug)).limit(1);
    if (!loc) error(404, 'Location not found');

    const [area] = await db.select({ id: locationAreas.id })
      .from(locationAreas)
      .where(and(eq(locationAreas.id, data.locationAreaId), eq(locationAreas.locationId, loc.id)))
      .limit(1);
    if (!area) return fail(400, { encError: 'Area does not belong to this location' });

    const [inserted] = await db.insert(encounters).values({
      locationAreaId:    data.locationAreaId,
      pokemonId:         data.pokemonId,
      versionId:         data.versionId,
      encounterMethodId: data.encounterMethodId,
      conditionValueId:  data.conditionValueId ?? null,
      minLevel:          data.minLevel,
      maxLevel:          data.maxLevel,
      chance:            data.chance,
      slot:              data.slot ?? null,
      source:            'custom',
    }).returning({ id: encounters.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'encounters', inserted.id, null, data);
    }

    return { encSuccess: true };
  },

  deleteEncounter: async ({ request, locals }) => {
    requireEditor(locals);
    const id = Number((await request.formData()).get('id'));
    if (!id) return fail(400, { error: 'Invalid' });

    const [enc] = await db.select({ id: encounters.id })
      .from(encounters).where(eq(encounters.id, id)).limit(1);
    if (!enc) return fail(404, { error: 'Encounter not found' });

    await db.delete(encounters).where(eq(encounters.id, id));
    return {};
  },

  delete: async ({ params, locals }) => {
    requireEditor(locals);

    const [loc] = await db.select({ id: locations.id, source: locations.source })
      .from(locations).where(eq(locations.slug, params.slug)).limit(1);
    if (!loc) error(404, 'Location not found');
    if (loc.source !== 'custom') return fail(400, { deleteError: 'Cannot delete PokeAPI location' });

    const areaIds = await db.select({ id: locationAreas.id })
      .from(locationAreas).where(eq(locationAreas.locationId, loc.id));

    if (areaIds.length > 0) {
      const [{ enc }] = await db.select({ enc: count() })
        .from(encounters)
        .where(inArray(encounters.locationAreaId, areaIds.map(a => a.id)));
      if (enc > 0) return fail(400, { deleteError: 'Cannot delete location with encounter data' });
    }

    await db.delete(locations).where(eq(locations.id, loc.id));
    redirect(303, '/editor/locations');
  },
};
