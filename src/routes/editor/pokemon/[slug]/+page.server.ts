import { error, fail, redirect } from '@sveltejs/kit';
import { z } from 'zod';
import fs from 'fs';
import path from 'path';
import { db } from '$lib/db';
import { pokemon, pokemonAbilities, pokemonEvolutionDetails, pokemonForms, pokemonGenera, pokemonMoves, pokemonSpecies, pokemonTypes, eggGroups, speciesEggGroups } from '$lib/db/schema/pokemon';
import { abilities, moveLearnMethods, moves, types } from '$lib/db/schema/core-data';
import { languages, versionGroups } from '$lib/db/schema/game-metadata';
import { items } from '$lib/db/schema/items';
import { and, asc, count, desc, eq, inArray, sql } from 'drizzle-orm';
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
  spriteArtworkShiny: z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteBackDefault:  z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  spriteBackShiny:    z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  slot1TypeId:    z.coerce.number().int().positive(),
  slot2TypeId:    z.preprocess((v) => (v === '' || v == null ? null : Number(v)), z.number().int().positive().nullable()),
  slot1AbilityId: z.coerce.number().int().positive(),
  slot2AbilityId: z.preprocess((v) => (v === '' || v == null ? null : Number(v)), z.number().int().positive().nullable()),
  hiddenAbilityId: z.preprocess((v) => (v === '' || v == null ? null : Number(v)), z.number().int().positive().nullable()),
});

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const textOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : String(v)),
  z.string().nullable(),
);

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

  // All pokemon of the same species (for form sub-tabs: ALOLA, MEGA, GMAX, etc.)
  const FORM_SKIP = ['-totem', '-cap', '-cosplay', '-rock-star', '-belle', '-pop-star', '-phd', '-libre', '-starter'];
  const speciesFormRows = await db.select({
    pokeapiId:    pokemon.pokeapiId,
    name:         pokemon.name,
    slug:         pokemon.slug,
    isDefault:    sql<boolean>`(${pokemon.id} = ${pokemonSpecies.defaultPokemonId})`,
    spriteDefault: pokemon.spriteDefault,
    spriteArtwork: pokemon.spriteArtwork,
    typeName:     types.name,
    typeHexColor: types.hexColor,
  })
    .from(pokemon)
    .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
    .innerJoin(pokemonTypes, eq(pokemonTypes.pokemonId, pokemon.id))
    .innerJoin(types, eq(pokemonTypes.typeId, types.id))
    .where(eq(pokemon.speciesId, row.speciesId))
    .orderBy(desc(sql<boolean>`(${pokemon.id} = ${pokemonSpecies.defaultPokemonId})`), asc(pokemon.name), asc(pokemonTypes.slot));

  const formMap = new Map<string, {
    pokeapiId: number | null; name: string; slug: string;
    isDefault: boolean; spriteUrl: string | null;
    types: { name: string; hexColor: string | null }[];
  }>();
  for (const r of speciesFormRows) {
    if (FORM_SKIP.some(p => r.slug.includes(p))) continue;
    if (!formMap.has(r.slug)) {
      formMap.set(r.slug, {
        pokeapiId: r.pokeapiId, name: r.name, slug: r.slug,
        isDefault: r.isDefault, spriteUrl: r.spriteArtwork ?? r.spriteDefault,
        types: [],
      });
    }
    formMap.get(r.slug)!.types.push({ name: r.typeName, hexColor: r.typeHexColor });
  }
  const speciesForms = [...formMap.values()];

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

  // Full evolution chain: walk up to root, BFS down, batch-load node data
  let evoRootId = row.id;
  {
    const seen = new Set<number>([evoRootId]);
    let cursor: number | null = row.evolvesFromPokemonId ?? null;
    while (cursor != null && !seen.has(cursor)) {
      seen.add(cursor);
      evoRootId = cursor;
      const [p] = await db.select({ e: pokemon.evolvesFromPokemonId })
        .from(pokemon).where(eq(pokemon.id, cursor)).limit(1);
      cursor = p?.e ?? null;
    }
  }
  const chainIds = new Set<number>([evoRootId]);
  {
    let frontier = [evoRootId];
    while (frontier.length > 0) {
      const children = await db.select({ id: pokemon.id })
        .from(pokemon).where(inArray(pokemon.evolvesFromPokemonId, frontier));
      frontier = children.filter(c => !chainIds.has(c.id)).map(c => c.id);
      frontier.forEach(id => chainIds.add(id));
    }
  }
  const allChainIds = [...chainIds];
  const evoChainRows = await db.select({
    id:            pokemon.id,
    pokeapiId:     pokemon.pokeapiId,
    slug:          pokemon.slug,
    name:          pokemon.name,
    spriteArtwork: pokemon.spriteArtwork,
    spriteDefault: pokemon.spriteDefault,
    evolvesFromId: pokemon.evolvesFromPokemonId,
    dexNumber:     pokemonSpecies.dexNumber,
    typeName:      types.name,
    typeHexColor:  types.hexColor,
  })
    .from(pokemon)
    .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
    .innerJoin(pokemonTypes, eq(pokemonTypes.pokemonId, pokemon.id))
    .innerJoin(types, eq(pokemonTypes.typeId, types.id))
    .where(inArray(pokemon.id, allChainIds))
    .orderBy(asc(pokemonSpecies.dexNumber), asc(pokemonTypes.slot));

  const chainCondRows = await db.select().from(pokemonEvolutionDetails)
    .where(inArray(pokemonEvolutionDetails.pokemonId, allChainIds));
  const chainItemIds = [...new Set(chainCondRows.map(c => c.itemId).filter((v): v is number => v != null))];
  const chainItems = chainItemIds.length > 0
    ? await db.select({ id: items.id, name: items.name, slug: items.slug })
        .from(items).where(inArray(items.id, chainItemIds))
    : [];
  const chainItemById = new Map(chainItems.map(i => [i.id, i]));
  const chainCondById = new Map(chainCondRows.map(c => [c.pokemonId, c]));

  const evoNodeMap = new Map<number, {
    id: number; pokeapiId: number; slug: string; name: string; spriteUrl: string | null;
    evolvesFromId: number | null; dexNumber: number;
    types: { name: string; hexColor: string | null }[];
    conditions: { trigger: string; minLevel: number | null; minHappiness: number | null;
                  itemName: string | null; itemSlug: string | null; timeOfDay: string | null; } | null;
  }>();
  for (const r of evoChainRows) {
    if (!evoNodeMap.has(r.id)) {
      const cond = chainCondById.get(r.id) ?? null;
      const item = cond?.itemId != null ? (chainItemById.get(cond.itemId) ?? null) : null;
      evoNodeMap.set(r.id, {
        id: r.id, pokeapiId: r.pokeapiId ?? 0, slug: r.slug, name: r.name,
        spriteUrl: r.spriteArtwork ?? r.spriteDefault,
        evolvesFromId: r.evolvesFromId ?? null,
        dexNumber: r.dexNumber, types: [],
        conditions: cond ? {
          trigger: cond.trigger, minLevel: cond.minLevel, minHappiness: cond.minHappiness,
          itemName: item?.name ?? null, itemSlug: item?.slug ?? null, timeOfDay: cond.timeOfDay,
        } : null,
      });
    }
    evoNodeMap.get(r.id)!.types.push({ name: r.typeName, hexColor: r.typeHexColor });
  }
  const evoChain = [...evoNodeMap.values()];

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
    typeName: string; typeHexColor: string | null; damageClass: string | null;
    learnMethodId: number; learnMethodName: string;
    level: number | null;
  }[] = [];
  if (selectedVgId != null) {
    learnset = await db.select({
      moveId:          pokemonMoves.moveId,
      moveName:        moves.name,
      moveSlug:        moves.slug,
      typeId:          moves.typeId,
      typeName:        types.name,
      typeHexColor:    types.hexColor,
      damageClass:     moves.damageClass,
      learnMethodId:   pokemonMoves.learnMethodId,
      learnMethodName: moveLearnMethods.name,
      level:           pokemonMoves.level,
    })
      .from(pokemonMoves)
      .innerJoin(moves, eq(pokemonMoves.moveId, moves.id))
      .innerJoin(types, eq(moves.typeId, types.id))
      .innerJoin(moveLearnMethods, eq(pokemonMoves.learnMethodId, moveLearnMethods.id))
      .where(and(eq(pokemonMoves.pokemonId, row.id), eq(pokemonMoves.versionGroupId, selectedVgId)))
      .orderBy(asc(pokemonMoves.learnMethodId), asc(pokemonMoves.level), asc(moves.name));
  }

  // Genus (English)
  const genusRow = await db.select({ genus: pokemonGenera.genus })
    .from(pokemonGenera)
    .innerJoin(languages, eq(pokemonGenera.languageId, languages.id))
    .where(and(
      eq(pokemonGenera.speciesId, row.speciesId),
      eq(languages.name, 'en')
    ))
    .limit(1);

  const genus = genusRow[0]?.genus ?? null;

  const [speciesRow] = await db.select()
    .from(pokemonSpecies)
    .where(eq(pokemonSpecies.id, row.speciesId))
    .limit(1);

  const currentEggGroups = await db.select({ id: eggGroups.id, name: eggGroups.name })
    .from(speciesEggGroups)
    .innerJoin(eggGroups, eq(speciesEggGroups.eggGroupId, eggGroups.id))
    .where(eq(speciesEggGroups.speciesId, row.speciesId));

  const allEggGroups = await db.select({ id: eggGroups.id, name: eggGroups.name })
    .from(eggGroups)
    .orderBy(asc(eggGroups.name));

  return {
    pokemon:    {
      ...row,
      isDefault: row.id === speciesRow?.defaultPokemonId,
    },
    types:      typeRows,
    abilities:  abilityRows,
    speciesForms,
    evoChain,
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
    genus,
    currentSpeciesDetail: speciesRow ?? null,
    currentEggGroups,
    allEggGroups,
  };
};

// Helper to save a file upload locally inside static/sprites/
async function saveUploadedFile(file: unknown, pokemonSlug: string, suffix: string): Promise<string | null> {
  if (!file || !(file instanceof File) || file.size === 0) {
    return null;
  }

  // Ensure output directory exists inside static
  const outputDir = path.join('static', 'sprites');
  fs.mkdirSync(outputDir, { recursive: true });

  // Get extension from original file name, fallback to png
  let ext = 'png';
  if (file.name) {
    const parsedExt = path.extname(file.name).toLowerCase();
    if (parsedExt) {
      ext = parsedExt.slice(1); // remove the leading dot
    }
  }

  // Clean the file extension (alphanumeric only)
  ext = ext.replace(/[^\w]/g, '') || 'png';

  const filename = `${pokemonSlug}-${suffix}.${ext}`;
  const filePath = path.join(outputDir, filename);

  // Convert File object to Buffer and write
  const arrayBuffer = await file.arrayBuffer();
  fs.writeFileSync(filePath, Buffer.from(arrayBuffer));

  // Return the public web-accessible URL path
  return `/sprites/${filename}`;
}

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    const formData = await request.formData();
    
    // Extract regular text/number fields to validate with Zod schema
    const form: Record<string, any> = {};
    for (const key of formData.keys()) {
      if (!key.endsWith('File')) {
        form[key] = formData.get(key);
      }
    }

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

    // Save uploaded files if any
    const defaultFile = formData.get('spriteDefaultFile');
    const shinyFile = formData.get('spriteShinyFile');
    const artworkFile = formData.get('spriteArtworkFile');
    const artworkShinyFile = formData.get('spriteArtworkShinyFile');
    const backDefaultFile = formData.get('spriteBackDefaultFile');
    const backShinyFile = formData.get('spriteBackShinyFile');

    let spriteDefault = data.spriteDefault;
    let spriteShiny = data.spriteShiny;
    let spriteArtwork = data.spriteArtwork;
    let spriteArtworkShiny = data.spriteArtworkShiny;
    let spriteBackDefault = data.spriteBackDefault;
    let spriteBackShiny = data.spriteBackShiny;

    const savedDefault = await saveUploadedFile(defaultFile, data.slug, 'default');
    if (savedDefault) spriteDefault = savedDefault;

    const savedShiny = await saveUploadedFile(shinyFile, data.slug, 'shiny');
    if (savedShiny) spriteShiny = savedShiny;

    const savedArtwork = await saveUploadedFile(artworkFile, data.slug, 'artwork');
    if (savedArtwork) spriteArtwork = savedArtwork;

    const savedArtworkShiny = await saveUploadedFile(artworkShinyFile, data.slug, 'artwork-shiny');
    if (savedArtworkShiny) spriteArtworkShiny = savedArtworkShiny;

    const savedBackDefault = await saveUploadedFile(backDefaultFile, data.slug, 'back-default');
    if (savedBackDefault) spriteBackDefault = savedBackDefault;

    const savedBackShiny = await saveUploadedFile(backShinyFile, data.slug, 'back-shiny');
    if (savedBackShiny) spriteBackShiny = savedBackShiny;

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
      spriteDefault,
      spriteShiny,
      spriteArtwork,
      spriteArtworkShiny,
      spriteBackDefault,
      spriteBackShiny,
      source:        bumpSource(current.source),
    };

    await db.update(pokemon).set(newRow).where(eq(pokemon.id, current.id));

    await db.update(pokemonForms)
      .set({
        spriteDefault: newRow.spriteDefault,
        spriteShiny: newRow.spriteShiny,
        spriteArtwork: newRow.spriteArtwork,
        spriteArtworkShiny: newRow.spriteArtworkShiny,
        spriteBackDefault: newRow.spriteBackDefault,
        spriteBackShiny: newRow.spriteBackShiny
      })
      .where(and(eq(pokemonForms.pokemonId, current.id), eq(pokemonForms.isDefault, true)));

    // Update types
    await db.delete(pokemonTypes).where(eq(pokemonTypes.pokemonId, current.id));
    await db.insert(pokemonTypes).values({ pokemonId: current.id, typeId: data.slot1TypeId, slot: 1 });
    if (data.slot2TypeId) {
      await db.insert(pokemonTypes).values({ pokemonId: current.id, typeId: data.slot2TypeId, slot: 2 });
    }

    // Update abilities
    await db.delete(pokemonAbilities).where(eq(pokemonAbilities.pokemonId, current.id));
    await db.insert(pokemonAbilities).values({ pokemonId: current.id, abilityId: data.slot1AbilityId, slot: 1, isHidden: false });
    if (data.slot2AbilityId && data.slot2AbilityId !== data.slot1AbilityId) {
      await db.insert(pokemonAbilities).values({ pokemonId: current.id, abilityId: data.slot2AbilityId, slot: 2, isHidden: false });
    }
    if (data.hiddenAbilityId && data.hiddenAbilityId !== data.slot1AbilityId && data.hiddenAbilityId !== data.slot2AbilityId) {
      await db.insert(pokemonAbilities).values({ pokemonId: current.id, abilityId: data.hiddenAbilityId, slot: 3, isHidden: true });
    }

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', current.id, current, { ...current, ...newRow });
    }

    if (data.slug !== current.slug) throw redirect(303, `/editor/pokemon/${data.slug}`);
    return { saved: true };
  },

  setFormBase: async ({ request, params, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const basePokemonId = Number(form.basePokemonId);
    const formType = String(form.formType || 'other');
    if (!Number.isInteger(basePokemonId) || basePokemonId <= 0) {
      return fail(400, { evoError: 'basePokemonId inválido' });
    }
    const [current] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    if (basePokemonId === current.id) return fail(400, { evoError: 'Cannot be a form of itself' });
    const [base] = await db.select({ id: pokemon.id }).from(pokemon)
      .where(eq(pokemon.id, basePokemonId)).limit(1);
    if (!base) return fail(400, { evoError: 'Base Pokémon not found' });
    await db.update(pokemon)
      .set({ formOfPokemonId: basePokemonId, formType, source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));
    return { evoSaved: true };
  },

  clearFormBase: async ({ params, locals }) => {
    const [current] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!current) throw error(404, 'Pokémon not found');
    await db.update(pokemon)
      .set({ formOfPokemonId: null, formType: null, source: bumpSource(current.source) })
      .where(eq(pokemon.id, current.id));
    return { evoSaved: true };
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

  addSuccessor: async ({ request, params, locals }) => {
    const formData = await request.formData();
    const childId = intOrNull.parse(formData.get('childPokemonId'));
    if (!childId) return fail(400, { evoError: 'childPokemonId required' });

    const [cur] = await db.select({ id: pokemon.id })
      .from(pokemon).where(eq(pokemon.slug, params.slug)).limit(1);
    if (!cur) throw error(404, 'Pokémon not found');

    const [child] = await db.select({ id: pokemon.id, source: pokemon.source })
      .from(pokemon).where(eq(pokemon.id, childId)).limit(1);
    if (!child) return fail(404, { evoError: 'Target pokemon not found' });

    await db.update(pokemon)
      .set({ evolvesFromPokemonId: cur.id, source: bumpSource(child.source) })
      .where(eq(pokemon.id, childId));
    await db.insert(pokemonEvolutionDetails)
      .values({ pokemonId: childId, trigger: 'level-up', source: 'edited' })
      .onConflictDoNothing();

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', childId, child, { evolvesFromPokemonId: cur.id });
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
