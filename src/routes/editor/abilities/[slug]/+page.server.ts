import { error, fail, redirect } from '@sveltejs/kit';
import { and, asc, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import { abilities } from '$lib/db/schema/core-data';
import { languages } from '$lib/db/schema/game-metadata';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const intOrNull = z.preprocess(
  (v) => {
    if (v === '' || v === null || v === undefined) return null;
    const n = Number(v);
    return Number.isFinite(n) ? n : v;
  },
  z.number().int().positive().nullable(),
);

const UpdateSchema = z.object({
  name:         z.string().trim().min(1).max(50),
  slug:         z.string().trim().min(1).max(50).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  isMainSeries: boolField,
  generationId: intOrNull,
});

const EffectsSchema = z.object({
  effect:      z.string().trim().min(0).max(4000),
  shortEffect: z.string().trim().min(0).max(2000),
});

export const load: PageServerLoad = async ({ params }) => {
  const [row] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Ability not found');

  // English language ID
  const [enLang] = await db.select({ id: languages.id })
    .from(languages).where(eq(languages.name, 'en')).limit(1);
  const englishId = enLang?.id ?? null;

  // Effect text
  const effectRow = englishId ? await db.select({
    shortEffect: schema.abilityEffects.shortEffect,
    effect: schema.abilityEffects.effect,
  })
  .from(schema.abilityEffects)
  .where(and(
    eq(schema.abilityEffects.abilityId, row.id),
    eq(schema.abilityEffects.languageId, englishId),
  ))
  .limit(1) : [];

  // Flavor texts per version group
  const flavorTexts = englishId ? await db.select({
    flavorText: schema.abilityFlavorTexts.flavorText,
    versionGroupId: schema.abilityFlavorTexts.versionGroupId,
    versionGroupName: schema.versionGroups.name,
  })
  .from(schema.abilityFlavorTexts)
  .innerJoin(schema.versionGroups, eq(schema.abilityFlavorTexts.versionGroupId, schema.versionGroups.id))
  .where(and(
    eq(schema.abilityFlavorTexts.abilityId, row.id),
    eq(schema.abilityFlavorTexts.languageId, englishId),
  ))
  .orderBy(asc(schema.versionGroups.order)) : [];

  // Pokemon with this ability
  const pokemonData = await db.select({
    id: schema.pokemon.id,
    pokeapiId: schema.pokemon.pokeapiId,
    name: schema.pokemon.name,
    slug: schema.pokemon.slug,
    spriteDefault: schema.pokemon.spriteDefault,
    isHidden: schema.pokemonAbilities.isHidden,
  })
  .from(schema.pokemonAbilities)
  .innerJoin(schema.pokemon, eq(schema.pokemonAbilities.pokemonId, schema.pokemon.id))
  .where(eq(schema.pokemonAbilities.abilityId, row.id))
  .orderBy(asc(schema.pokemon.pokeapiId));

  const typesMap = new Map<number, Array<{ name: string; hexColor: string | null }>>();

  if (pokemonData.length > 0) {
    const pTypes = await db.select({
      pokemonId: schema.pokemonTypes.pokemonId,
      typeName: schema.types.name,
      typeHexColor: schema.types.hexColor,
    })
    .from(schema.pokemonTypes)
    .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
    .orderBy(asc(schema.pokemonTypes.slot));

    for (const pt of pTypes) {
      if (!typesMap.has(pt.pokemonId)) {
        typesMap.set(pt.pokemonId, []);
      }
      typesMap.get(pt.pokemonId)!.push({ name: pt.typeName, hexColor: pt.typeHexColor });
    }
  }

  const pokemonList = pokemonData.map(p => ({
    ...p,
    types: typesMap.get(p.id) || [],
  }));

  return {
    ability:       row,
    pokemonList,
    flavorTexts,
    shortEffect:   effectRow[0]?.shortEffect ?? null,
    effect:        effectRow[0]?.effect ?? null,
    englishLangId: englishId,
  };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);

    if (!parsed.success) {
      return fail(400, {
        updateError: parsed.error.issues[0]?.message ?? 'Inválido',
        values: form,
      });
    }

    const data = parsed.data;
    const [current] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: abilities.id })
        .from(abilities).where(eq(abilities.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:         data.name,
      slug:         data.slug,
      isMainSeries: data.isMainSeries,
      generationId: data.generationId ?? null,
      source:       bumpSource(current.source),
    };

    await db.update(abilities).set(newRow).where(eq(abilities.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/abilities/${data.slug}`);
    return { saved: true };
  },

  updateEffects: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = EffectsSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { effectsError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: abilities.id })
      .from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');

    const [enLang] = await db.select({ id: languages.id })
      .from(languages).where(eq(languages.name, 'en')).limit(1);
    if (!enLang) return fail(400, { effectsError: 'English language row not found' });

    const [previous] = await db.select({
      effect:      schema.abilityEffects.effect,
      shortEffect: schema.abilityEffects.shortEffect,
    })
      .from(schema.abilityEffects)
      .where(and(
        eq(schema.abilityEffects.abilityId, current.id),
        eq(schema.abilityEffects.languageId, enLang.id),
      )).limit(1);

    await db.insert(schema.abilityEffects).values({
      abilityId:   current.id,
      languageId:  enLang.id,
      effect:      data.effect,
      shortEffect: data.shortEffect,
    }).onConflictDoUpdate({
      target: [schema.abilityEffects.abilityId, schema.abilityEffects.languageId],
      set:    { effect: data.effect, shortEffect: data.shortEffect },
    });

    if (locals.user) {
      await recordEdit(
        locals.user.id,
        'ability_effects',
        current.id,
        previous ?? null,
        { effect: data.effect, shortEffect: data.shortEffect },
      );
    }

    return { effectsSaved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas registros edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(abilities).set({ source: 'pokeapi' }).where(eq(abilities.id, current.id));
    try {
      await reimportEntity('abilities', current.pokeapiId);
    } catch (err) {
      await db.update(abilities).set({ source: 'edited' }).where(eq(abilities.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(abilities).where(eq(abilities.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Ability not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas registros source=custom podem ser deletados' });
    }

    await db.delete(abilities).where(eq(abilities.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'abilities', current.id, current, null);
    }

    throw redirect(303, '/editor/abilities');
  },
};
