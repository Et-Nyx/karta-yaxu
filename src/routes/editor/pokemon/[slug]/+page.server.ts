import { error, fail, redirect } from '@sveltejs/kit';
import { z } from 'zod';
import { db } from '$lib/db';
import { pokemon, pokemonAbilities, pokemonEvolutionDetails, pokemonForms, pokemonMoves, pokemonSpecies, pokemonTypes } from '$lib/db/schema/pokemon';
import { abilities, moveLearnMethods, moves, types } from '$lib/db/schema/core-data';
import { versionGroups } from '$lib/db/schema/game-metadata';
import { items } from '$lib/db/schema/items';
import { and, asc, count, eq, inArray, sql } from 'drizzle-orm';
import { bumpSource } from '$lib/editor/source';
import { recordEdit } from '$lib/editor/audit';
import { reimportEntity } from '$lib/editor/resource';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const intOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
  z.number().int().nullable(),
);

const stat = z.coerce.number().int().min(0).max(255);

const UpdateSchema = z.object({
  name:          z.string().trim().min(1).max(80),
  slug:          z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  speciesId:     z.coerce.number().int().positive(),
  height:        z.coerce.number().int().min(0),
  weight:        z.coerce.number().int().min(0),
  hp:            stat,
  atk:           stat,
  def:           stat,
  spa:           stat,
  spd:           stat,
  spe:           stat,
  spriteDefault: z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteShiny:   z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteArtwork: z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  isDefault:     z.preprocess((v) => v === 'true' || v === 'on' || v === true, z.boolean()),
  order:         intOrNull,
});

const TypesSchema = z.object({
  slot1TypeId: z.coerce.number().int().positive(),
  slot2TypeId: z.preprocess(
    (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
    z.number().int().positive().nullable(),
  ),
});

const AbilitiesSchema = z.object({
  slot1AbilityId:  z.coerce.number().int().positive(),
  slot2AbilityId:  z.preprocess(
    (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
    z.number().int().positive().nullable(),
  ),
  hiddenAbilityId: z.preprocess(
    (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
    z.number().int().positive().nullable(),
  ),
});

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const textOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : String(v)),
  z.string().nullable(),
);

const FormSchema = z.object({
  formName:      textOrNull,
  isDefault:     boolField,
  isBattleOnly:  boolField,
  isMega:        boolField,
  order:         intOrNull,
  spriteDefault: textOrNull,
  spriteShiny:   textOrNull,
  spriteArtwork: textOrNull,
});

const EVO_TRIGGERS = ['level-up', 'use-item', 'use-move', 'trade', 'three-critical-hits', 'shed', 'spin', 'tower-of-darkness', 'tower-of-waters', 'agile-style-move', 'strong-style-move'] as const;

const SetPredecessorSchema = z.object({
  parentPokemonId: z.coerce.number().int().positive(),
  trigger:         z.enum(EVO_TRIGGERS),
  minLevel:        intOrNull,
  itemId:          intOrNull,
  heldItemId:      intOrNull,
  knownMoveId:     intOrNull,
  knownMoveTypeId: intOrNull,
  minHappiness:    intOrNull,
  timeOfDay:       textOrNull,
  gender:          intOrNull,
  location:        textOrNull,
});

const UpdateConditionsSchema = SetPredecessorSchema.omit({ parentPokemonId: true });

const LearnsetEntrySchema = z.object({
  moveId:         z.coerce.number().int().positive(),
  learnMethodId:  z.coerce.number().int().positive(),
  level:          z.coerce.number().int().min(0).max(100).default(0),
  versionGroupId: z.coerce.number().int().positive(),
});

const LearnsetUpdateSchema = z.object({
  oldMoveId:         z.coerce.number().int().positive(),
  oldLearnMethodId:  z.coerce.number().int().positive(),
  oldLevel:          z.coerce.number().int().min(0),
  oldVersionGroupId: z.coerce.number().int().positive(),
  newMoveId:         z.coerce.number().int().positive(),
  newLearnMethodId:  z.coerce.number().int().positive(),
  newLevel:          z.coerce.number().int().min(0).max(100),
  newVersionGroupId: z.coerce.number().int().positive(),
});

export const load: PageServerLoad = async ({ params, url }) => {
  const [row] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
  if (!row) throw error(404, 'Pokémon not found');

  const typeRows = await db.select({
    slot:     pokemonTypes.slot,
    typeId:   pokemonTypes.typeId,
    typeName: types.name,
    typeSlug: types.slug,
    hexColor: types.hexColor,
  })
    .from(pokemonTypes)
    .innerJoin(types, eq(pokemonTypes.typeId, types.id))
    .where(eq(pokemonTypes.pokemonId, row.id))
    .orderBy(asc(pokemonTypes.slot));

  const abilityRows = await db.select({
    abilityId:   pokemonAbilities.abilityId,
    slot:        pokemonAbilities.slot,
    isHidden:    pokemonAbilities.isHidden,
    abilityName: abilities.name,
    abilitySlug: abilities.slug,
  })
    .from(pokemonAbilities)
    .innerJoin(abilities, eq(pokemonAbilities.abilityId, abilities.id))
    .where(eq(pokemonAbilities.pokemonId, row.id))
    .orderBy(asc(pokemonAbilities.slot));

  const speciesList = await db.select({
    id:        pokemonSpecies.id,
    name:      pokemonSpecies.name,
    slug:      pokemonSpecies.slug,
    dexNumber: pokemonSpecies.dexNumber,
  }).from(pokemonSpecies).orderBy(asc(pokemonSpecies.dexNumber));

  const typeList = await db.select({
    id:       types.id,
    name:     types.name,
    slug:     types.slug,
    hexColor: types.hexColor,
  }).from(types).orderBy(asc(types.id));

  const abilityList = await db.select({
    id:   abilities.id,
    name: abilities.name,
    slug: abilities.slug,
  }).from(abilities).orderBy(asc(abilities.name));

  const formRows = await db.select().from(pokemonForms)
    .where(eq(pokemonForms.pokemonId, row.id))
    .orderBy(asc(pokemonForms.order), asc(pokemonForms.id));

  // Evolution data: predecessor + its conditions, and list of successors
  let predecessor: { id: number; slug: string; name: string } | null = null;
  let predecessorConditions: typeof pokemonEvolutionDetails.$inferSelect | null = null;
  if (row.evolvesFromPokemonId) {
    const [parent] = await db.select({ id: pokemon.id, slug: pokemon.slug, name: pokemon.name })
      .from(pokemon).where(eq(pokemon.id, row.evolvesFromPokemonId)).limit(1);
    if (parent) predecessor = parent;
    const [cond] = await db.select().from(pokemonEvolutionDetails)
      .where(eq(pokemonEvolutionDetails.pokemonId, row.id)).limit(1);
    if (cond) predecessorConditions = cond;
  }

  const successorRows = await db.select({
    id:       pokemon.id,
    slug:     pokemon.slug,
    name:     pokemon.name,
    trigger:  pokemonEvolutionDetails.trigger,
    minLevel: pokemonEvolutionDetails.minLevel,
    itemId:   pokemonEvolutionDetails.itemId,
  })
    .from(pokemon)
    .leftJoin(pokemonEvolutionDetails, eq(pokemonEvolutionDetails.pokemonId, pokemon.id))
    .where(eq(pokemon.evolvesFromPokemonId, row.id))
    .orderBy(asc(pokemon.id));

  // FK dropdowns
  const allPokemon = await db.select({ id: pokemon.id, slug: pokemon.slug, name: pokemon.name })
    .from(pokemon).orderBy(asc(pokemon.name));
  const allItems = await db.select({ id: items.id, name: items.name, slug: items.slug })
    .from(items).orderBy(asc(items.name));
  const allMoves = await db.select({ id: moves.id, name: moves.name, slug: moves.slug, typeId: moves.typeId })
    .from(moves).orderBy(asc(moves.name));

  // Learnset support data
  const vgCounts = await db.select({
    id:    versionGroups.id,
    slug:  versionGroups.slug,
    name:  versionGroups.name,
    count: count(pokemonMoves.moveId),
  })
    .from(versionGroups)
    .leftJoin(pokemonMoves, and(
      eq(pokemonMoves.versionGroupId, versionGroups.id),
      eq(pokemonMoves.pokemonId, row.id),
    ))
    .groupBy(versionGroups.id)
    .orderBy(asc(versionGroups.id));

  const allLearnMethods = await db.select({
    id:   moveLearnMethods.id,
    slug: moveLearnMethods.slug,
    name: moveLearnMethods.name,
  }).from(moveLearnMethods).orderBy(asc(moveLearnMethods.id));

  // Pick the active version group from ?vg=… or the most populated for this pokemon
  let selectedVgId: number | null = null;
  const vgParam = url.searchParams.get('vg');
  if (vgParam) {
    const match = vgCounts.find(v => v.slug === vgParam);
    if (match) selectedVgId = match.id;
  }
  if (selectedVgId == null) {
    const best = vgCounts.filter(v => v.count > 0).sort((a, b) => b.count - a.count)[0];
    if (best) selectedVgId = best.id;
    else if (vgCounts.length > 0) selectedVgId = vgCounts[0].id;
  }

  let learnset: {
    moveId: number; moveName: string; moveSlug: string; typeId: number;
    learnMethodId: number; learnMethodName: string;
    level: number | null;
  }[] = [];
  if (selectedVgId != null) {
    learnset = await db.select({
      moveId:          pokemonMoves.moveId,
      moveName:        moves.name,
      moveSlug:        moves.slug,
      typeId:          moves.typeId,
      learnMethodId:   pokemonMoves.learnMethodId,
      learnMethodName: moveLearnMethods.name,
      level:           pokemonMoves.level,
    })
      .from(pokemonMoves)
      .innerJoin(moves, eq(pokemonMoves.moveId, moves.id))
      .innerJoin(moveLearnMethods, eq(pokemonMoves.learnMethodId, moveLearnMethods.id))
      .where(and(eq(pokemonMoves.pokemonId, row.id), eq(pokemonMoves.versionGroupId, selectedVgId)))
      .orderBy(asc(pokemonMoves.learnMethodId), asc(pokemonMoves.level), asc(moves.name));
  }

  return {
    pokemon:    row,
    types:      typeRows,
    abilities:  abilityRows,
    forms:      formRows,
    species:    speciesList,
    allTypes:   typeList,
    allAbilities: abilityList,
    predecessor,
    predecessorConditions,
    successors: successorRows,
    allPokemon,
    allItems,
    allMoves,
    vgCounts,
    allLearnMethods,
    selectedVgId,
    learnset,
  };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { updateError: parsed.error.issues[0]?.message ?? 'Inválido', values: form });
    }
    const data = parsed.data;

    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    if (data.slug !== current.slug) {
      const [clash] = await db.select({ id: pokemon.id })
        .from(pokemon).where(eq(pokemon.slug, data.slug)).limit(1);
      if (clash) return fail(400, { updateError: 'slug já em uso', values: form });
    }

    const newRow = {
      name:          data.name,
      slug:          data.slug,
      speciesId:     data.speciesId,
      height:        data.height,
      weight:        data.weight,
      hp:            data.hp,
      atk:           data.atk,
      def:           data.def,
      spa:           data.spa,
      spd:           data.spd,
      spe:           data.spe,
      spriteDefault: data.spriteDefault,
      spriteShiny:   data.spriteShiny,
      spriteArtwork: data.spriteArtwork,
      isDefault:     data.isDefault,
      order:         data.order,
      source:        bumpSource(current.source),
    };

    await db.update(pokemon).set(newRow).where(eq(pokemon.id, current.id));

    await db.update(pokemonForms)
      .set({ spriteDefault: newRow.spriteDefault, spriteShiny: newRow.spriteShiny, spriteArtwork: newRow.spriteArtwork })
      .where(and(eq(pokemonForms.pokemonId, current.id), eq(pokemonForms.isDefault, true)));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/pokemon/${data.slug}`);
    return { saved: true };
  },

  updateTypes: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = TypesSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { typesError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const previous = await db.select().from(pokemonTypes)
      .where(eq(pokemonTypes.pokemonId, current.id));

    await db.delete(pokemonTypes).where(eq(pokemonTypes.pokemonId, current.id));
    await db.insert(pokemonTypes).values({
      pokemonId: current.id,
      typeId:    data.slot1TypeId,
      slot:      1,
    });
    if (data.slot2TypeId) {
      await db.insert(pokemonTypes).values({
        pokemonId: current.id,
        typeId:    data.slot2TypeId,
        slot:      2,
      });
    }

    await db.update(pokemon)
      .set({ source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));

    if (locals.user) {
      const next = [
        { pokemonId: current.id, typeId: data.slot1TypeId, slot: 1 },
        ...(data.slot2TypeId ? [{ pokemonId: current.id, typeId: data.slot2TypeId, slot: 2 }] : []),
      ];
      await recordEdit(locals.user.id, 'pokemon_types', current.id, previous, next);
    }

    return { typesSaved: true };
  },

  updateAbilities: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = AbilitiesSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { abilitiesError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const previous = await db.select().from(pokemonAbilities)
      .where(eq(pokemonAbilities.pokemonId, current.id));

    await db.delete(pokemonAbilities).where(eq(pokemonAbilities.pokemonId, current.id));
    await db.insert(pokemonAbilities).values({
      pokemonId: current.id,
      abilityId: data.slot1AbilityId,
      slot:      1,
      isHidden:  false,
    });
    if (data.slot2AbilityId && data.slot2AbilityId !== data.slot1AbilityId) {
      await db.insert(pokemonAbilities).values({
        pokemonId: current.id,
        abilityId: data.slot2AbilityId,
        slot:      2,
        isHidden:  false,
      });
    }
    if (data.hiddenAbilityId
        && data.hiddenAbilityId !== data.slot1AbilityId
        && data.hiddenAbilityId !== data.slot2AbilityId) {
      await db.insert(pokemonAbilities).values({
        pokemonId: current.id,
        abilityId: data.hiddenAbilityId,
        slot:      3,
        isHidden:  true,
      });
    }

    await db.update(pokemon)
      .set({ source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));

    if (locals.user) {
      const next: any[] = [
        { pokemonId: current.id, abilityId: data.slot1AbilityId, slot: 1, isHidden: false },
      ];
      if (data.slot2AbilityId && data.slot2AbilityId !== data.slot1AbilityId) {
        next.push({ pokemonId: current.id, abilityId: data.slot2AbilityId, slot: 2, isHidden: false });
      }
      if (data.hiddenAbilityId
          && data.hiddenAbilityId !== data.slot1AbilityId
          && data.hiddenAbilityId !== data.slot2AbilityId) {
        next.push({ pokemonId: current.id, abilityId: data.hiddenAbilityId, slot: 3, isHidden: true });
      }
      await recordEdit(locals.user.id, 'pokemon_abilities', current.id, previous, next);
    }

    return { abilitiesSaved: true };
  },

  addForm: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = FormSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { formError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id, speciesId: pokemon.speciesId })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const [inserted] = await db.insert(pokemonForms).values({
      pokemonId:     current.id,
      speciesId:     current.speciesId,
      formName:      data.formName,
      isDefault:     data.isDefault,
      isBattleOnly:  data.isBattleOnly,
      isMega:        data.isMega,
      order:         data.order,
      spriteDefault: data.spriteDefault,
      spriteShiny:   data.spriteShiny,
      spriteArtwork: data.spriteArtwork,
      source:        'custom',
      pokeapiId:     null,
    }).returning({ id: pokemonForms.id });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_forms', inserted.id, null, data);
    }

    return { formSaved: true };
  },

  updateForm: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const formId = Number(form.formId);
    if (!Number.isInteger(formId) || formId <= 0) {
      return fail(400, { formError: 'formId inválido' });
    }

    const parsed = FormSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { formError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select().from(pokemonForms).where(eq(pokemonForms.id, formId)).limit(1);
    if (!current) return fail(404, { formError: 'Form não encontrado' });

    const newRow = {
      formName:      data.formName,
      isDefault:     data.isDefault,
      isBattleOnly:  data.isBattleOnly,
      isMega:        data.isMega,
      order:         data.order,
      spriteDefault: data.spriteDefault,
      spriteShiny:   data.spriteShiny,
      spriteArtwork: data.spriteArtwork,
      source:        bumpSource(current.source),
    };

    await db.update(pokemonForms).set(newRow).where(eq(pokemonForms.id, formId));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_forms', formId, current, { ...current, ...newRow });
    }

    return { formSaved: true };
  },

  deleteForm: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const formId = Number(form.formId);
    if (!Number.isInteger(formId) || formId <= 0) {
      return fail(400, { formError: 'formId inválido' });
    }

    const [current] = await db.select().from(pokemonForms).where(eq(pokemonForms.id, formId)).limit(1);
    if (!current) return fail(404, { formError: 'Form não encontrado' });
    if (current.source !== 'custom') {
      return fail(400, { formError: 'Apenas forms custom podem ser deletados' });
    }

    // Refuse if deleting would leave the pokemon without a default form
    if (current.isDefault) {
      const remaining = await db.select({ id: pokemonForms.id, isDefault: pokemonForms.isDefault })
        .from(pokemonForms)
        .where(eq(pokemonForms.pokemonId, current.pokemonId));
      const hasOtherDefault = remaining.some(r => r.id !== formId && r.isDefault);
      const wouldBeEmpty = remaining.length <= 1;
      if (!hasOtherDefault && !wouldBeEmpty) {
        return fail(400, { formError: 'Não é possível deletar o único form default — marque outro como default antes' });
      }
    }

    await db.delete(pokemonForms).where(eq(pokemonForms.id, formId));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_forms', formId, current, null);
    }

    return { formSaved: true };
  },

  setPredecessor: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = SetPredecessorSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { evoError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (data.parentPokemonId === current.id) {
      return fail(400, { evoError: 'Um Pokémon não pode evoluir de si mesmo' });
    }

    const [parent] = await db.select({ id: pokemon.id }).from(pokemon)
      .where(eq(pokemon.id, data.parentPokemonId)).limit(1);
    if (!parent) return fail(400, { evoError: 'Pokémon predecessor inválido' });

    const previousCond = await db.select().from(pokemonEvolutionDetails)
      .where(eq(pokemonEvolutionDetails.pokemonId, current.id)).limit(1);

    await db.update(pokemon)
      .set({ evolvesFromPokemonId: parent.id, source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));

    const conditionRow = {
      pokemonId:       current.id,
      trigger:         data.trigger,
      minLevel:        data.minLevel,
      itemId:          data.itemId,
      heldItemId:      data.heldItemId,
      knownMoveId:     data.knownMoveId,
      knownMoveTypeId: data.knownMoveTypeId,
      minHappiness:    data.minHappiness,
      timeOfDay:       data.timeOfDay,
      gender:          data.gender,
      location:        data.location,
      source:          'edited' as const,
    };

    await db.insert(pokemonEvolutionDetails).values(conditionRow).onConflictDoUpdate({
      target: pokemonEvolutionDetails.pokemonId,
      set: {
        trigger:         data.trigger,
        minLevel:        data.minLevel,
        itemId:          data.itemId,
        heldItemId:      data.heldItemId,
        knownMoveId:     data.knownMoveId,
        knownMoveTypeId: data.knownMoveTypeId,
        minHappiness:    data.minHappiness,
        timeOfDay:       data.timeOfDay,
        gender:          data.gender,
        location:        data.location,
        source:          'edited',
      },
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_evolution_details', current.id, previousCond[0] ?? null, conditionRow);
    }

    return { evoSaved: true };
  },

  clearPredecessor: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const previousCond = await db.select().from(pokemonEvolutionDetails)
      .where(eq(pokemonEvolutionDetails.pokemonId, current.id)).limit(1);

    await db.update(pokemon)
      .set({ evolvesFromPokemonId: null, source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));
    await db.delete(pokemonEvolutionDetails).where(eq(pokemonEvolutionDetails.pokemonId, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_evolution_details', current.id, previousCond[0] ?? null, null);
    }

    return { evoSaved: true };
  },

  updateConditions: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = UpdateConditionsSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { evoError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id, evo: pokemon.evolvesFromPokemonId, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (current.evo == null) return fail(400, { evoError: 'Defina um predecessor antes de editar condições' });

    const previousCond = await db.select().from(pokemonEvolutionDetails)
      .where(eq(pokemonEvolutionDetails.pokemonId, current.id)).limit(1);

    await db.update(pokemon)
      .set({ source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));

    await db.insert(pokemonEvolutionDetails).values({
      pokemonId:       current.id,
      trigger:         data.trigger,
      minLevel:        data.minLevel,
      itemId:          data.itemId,
      heldItemId:      data.heldItemId,
      knownMoveId:     data.knownMoveId,
      knownMoveTypeId: data.knownMoveTypeId,
      minHappiness:    data.minHappiness,
      timeOfDay:       data.timeOfDay,
      gender:          data.gender,
      location:        data.location,
      source:          'edited',
    }).onConflictDoUpdate({
      target: pokemonEvolutionDetails.pokemonId,
      set: {
        trigger:         data.trigger,
        minLevel:        data.minLevel,
        itemId:          data.itemId,
        heldItemId:      data.heldItemId,
        knownMoveId:     data.knownMoveId,
        knownMoveTypeId: data.knownMoveTypeId,
        minHappiness:    data.minHappiness,
        timeOfDay:       data.timeOfDay,
        gender:          data.gender,
        location:        data.location,
        source:          'edited',
      },
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_evolution_details', current.id, previousCond[0] ?? null, data);
    }

    return { evoSaved: true };
  },

  addLearnsetEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = LearnsetEntrySchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { learnsetError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const [exists] = await db.select({ moveId: pokemonMoves.moveId })
      .from(pokemonMoves)
      .where(and(
        eq(pokemonMoves.pokemonId, current.id),
        eq(pokemonMoves.moveId, data.moveId),
        eq(pokemonMoves.learnMethodId, data.learnMethodId),
        eq(pokemonMoves.level, data.level),
        eq(pokemonMoves.versionGroupId, data.versionGroupId),
      )).limit(1);
    if (exists) return fail(400, { learnsetError: 'Entrada já existe nessa combinação' });

    await db.insert(pokemonMoves).values({
      pokemonId:      current.id,
      moveId:         data.moveId,
      learnMethodId:  data.learnMethodId,
      level:          data.level,
      versionGroupId: data.versionGroupId,
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, null, data);
    }

    return { learnsetSaved: true };
  },

  removeLearnsetEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = LearnsetEntrySchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { learnsetError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    await db.delete(pokemonMoves).where(and(
      eq(pokemonMoves.pokemonId, current.id),
      eq(pokemonMoves.moveId, data.moveId),
      eq(pokemonMoves.learnMethodId, data.learnMethodId),
      eq(pokemonMoves.level, data.level),
      eq(pokemonMoves.versionGroupId, data.versionGroupId),
    ));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, data, null);
    }

    return { learnsetSaved: true };
  },

  updateLearnsetEntry: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = LearnsetUpdateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { learnsetError: parsed.error.issues[0]?.message ?? 'Inválido' });
    }
    const data = parsed.data;

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    db.transaction((tx) => {
      tx.delete(pokemonMoves).where(and(
        eq(pokemonMoves.pokemonId, current.id),
        eq(pokemonMoves.moveId, data.oldMoveId),
        eq(pokemonMoves.learnMethodId, data.oldLearnMethodId),
        eq(pokemonMoves.level, data.oldLevel),
        eq(pokemonMoves.versionGroupId, data.oldVersionGroupId),
      )).run();
      tx.insert(pokemonMoves).values({
        pokemonId:      current.id,
        moveId:         data.newMoveId,
        learnMethodId:  data.newLearnMethodId,
        level:          data.newLevel,
        versionGroupId: data.newVersionGroupId,
      }).onConflictDoNothing().run();
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id,
        { moveId: data.oldMoveId, learnMethodId: data.oldLearnMethodId, level: data.oldLevel, versionGroupId: data.oldVersionGroupId },
        { moveId: data.newMoveId, learnMethodId: data.newLearnMethodId, level: data.newLevel, versionGroupId: data.newVersionGroupId },
      );
    }

    return { learnsetSaved: true };
  },

  clearLearnsetForVg: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const vgId = Number(form.versionGroupId);
    if (!Number.isInteger(vgId) || vgId <= 0) {
      return fail(400, { learnsetError: 'versionGroupId inválido' });
    }

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const [{ c: removed }] = await db.select({ c: count() })
      .from(pokemonMoves)
      .where(and(eq(pokemonMoves.pokemonId, current.id), eq(pokemonMoves.versionGroupId, vgId)));

    await db.delete(pokemonMoves).where(and(
      eq(pokemonMoves.pokemonId, current.id),
      eq(pokemonMoves.versionGroupId, vgId),
    ));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, { clearedVgId: vgId, removed }, null);
    }

    return { learnsetSaved: true };
  },

  copyLearnsetFromVg: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const sourceVgId = Number(form.sourceVgId);
    const targetVgId = Number(form.targetVgId);
    if (!Number.isInteger(sourceVgId) || sourceVgId <= 0 || !Number.isInteger(targetVgId) || targetVgId <= 0) {
      return fail(400, { learnsetError: 'IDs de version group inválidos' });
    }
    if (sourceVgId === targetVgId) {
      return fail(400, { learnsetError: 'Source e target precisam ser diferentes' });
    }

    const [current] = await db.select({ id: pokemon.id }).from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');

    const sourceRows = await db.select({
      moveId:        pokemonMoves.moveId,
      learnMethodId: pokemonMoves.learnMethodId,
      level:         pokemonMoves.level,
    })
      .from(pokemonMoves)
      .where(and(eq(pokemonMoves.pokemonId, current.id), eq(pokemonMoves.versionGroupId, sourceVgId)));

    if (sourceRows.length === 0) {
      return fail(400, { learnsetError: 'Source version group não tem nenhuma entrada' });
    }

    let copied = 0;
    db.transaction((tx) => {
      for (const r of sourceRows) {
        const result = tx.insert(pokemonMoves).values({
          pokemonId:      current.id,
          moveId:         r.moveId,
          learnMethodId:  r.learnMethodId,
          level:          r.level,
          versionGroupId: targetVgId,
        }).onConflictDoNothing().run();
        if (result.changes > 0) copied++;
      }
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_moves', current.id, null, { copiedFromVg: sourceVgId, toVg: targetVgId, copied });
    }

    return { learnsetSaved: true };
  },

  resource: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (current.source !== 'edited') return fail(400, { updateError: 'Apenas pokémon edited podem ser re-sourceados' });
    if (current.pokeapiId == null) return fail(400, { updateError: 'Sem pokeapiId — não há fonte para re-importar' });

    await db.update(pokemon).set({ source: 'pokeapi' }).where(eq(pokemon.id, current.id));
    try {
      await reimportEntity('pokemon', current.pokeapiId);
    } catch (err) {
      await db.update(pokemon).set({ source: 'edited' }).where(eq(pokemon.id, current.id));
      return fail(500, { updateError: `Falha ao re-importar: ${(err as Error).message}` });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, { resourced: true, pokeapiId: current.pokeapiId });
    }
    return { resourced: true };
  },

  delete: async ({ params, locals }) => {
    const [current] = await db.select().from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (current.source !== 'custom') {
      return fail(400, { updateError: 'Apenas pokémon custom podem ser deletados' });
    }

    await db.delete(pokemonTypes).where(eq(pokemonTypes.pokemonId, current.id));
    await db.delete(pokemonAbilities).where(eq(pokemonAbilities.pokemonId, current.id));
    await db.delete(pokemonForms).where(eq(pokemonForms.pokemonId, current.id));
    await db.delete(pokemonEvolutionDetails).where(eq(pokemonEvolutionDetails.pokemonId, current.id));
    await db.delete(pokemonMoves).where(eq(pokemonMoves.pokemonId, current.id));
    await db.delete(pokemon).where(eq(pokemon.id, current.id));

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, null);
    }

    throw redirect(303, '/editor/pokemon');
  },
};
