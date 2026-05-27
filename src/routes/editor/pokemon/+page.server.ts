import { fail, redirect } from '@sveltejs/kit';
import { and, asc, count, desc, eq, sql } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import {
  pokemon,
  pokemonAbilities,
  pokemonForms,
  pokemonMoves,
  pokemonSpecies,
  pokemonTypes,
  pokedexEntries,
} from '$lib/db/schema/pokemon';
import { types } from '$lib/db/schema/core-data';
import { pokedexes } from '$lib/db/schema/game-metadata';
import { recordEdit } from '$lib/editor/audit';
import { sortLinkedListEntries } from '$lib/editor/pokedex';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const CreateSchema = z.object({
  name:      z.string().trim().min(1).max(80),
  slug:      z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  speciesId: z.coerce.number().int().positive(),
  height:    z.coerce.number().int().min(0),
  weight:    z.coerce.number().int().min(0),
  hp:        z.coerce.number().int().min(0).max(255),
  atk:       z.coerce.number().int().min(0).max(255),
  def:       z.coerce.number().int().min(0).max(255),
  spa:       z.coerce.number().int().min(0).max(255),
  spd:       z.coerce.number().int().min(0).max(255),
  spe:       z.coerce.number().int().min(0).max(255),
});

export const load: PageServerLoad = async () => {
  const rows = await db.select({
    id:            pokemon.id,
    name:          pokemon.name,
    slug:          pokemon.slug,
    speciesId:     pokemon.speciesId,
    hp:            pokemon.hp,
    atk:           pokemon.atk,
    def:           pokemon.def,
    spa:           pokemon.spa,
    spd:           pokemon.spd,
    spe:           pokemon.spe,
    spriteDefault: pokemon.spriteDefault,
    source:        pokemon.source,
    pokeapiId:     pokemon.pokeapiId,
    dexNumber:     pokemonSpecies.dexNumber,
  })
    .from(pokemon)
    .innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id))
    .orderBy(
      asc(pokemonSpecies.dexOrder),
      desc(sql<boolean>`(${pokemon.id} = ${pokemonSpecies.defaultPokemonId})`),
      asc(pokemon.name)
    );

  const speciesList = await db.select({
    id:        pokemonSpecies.id,
    name:      pokemonSpecies.name,
    slug:      pokemonSpecies.slug,
    dexNumber: pokemonSpecies.dexNumber,
  }).from(pokemonSpecies).orderBy(asc(pokemonSpecies.dexNumber));

  const typeRows = await db.select({
    pokemonId: pokemonTypes.pokemonId,
    name:      types.name,
    hexColor:  types.hexColor,
    slot:      pokemonTypes.slot,
  })
    .from(pokemonTypes)
    .innerJoin(types, eq(pokemonTypes.typeId, types.id))
    .orderBy(asc(pokemonTypes.pokemonId), asc(pokemonTypes.slot));

  const [{ total }] = await db.select({ total: count() }).from(pokemon);

  // 1. Fetch all available pokedexes
  const pokedexesList = await db.select({
    id: pokedexes.id,
    name: pokedexes.name,
    slug: pokedexes.slug,
    source: pokedexes.source,
  })
  .from(pokedexes)
  .orderBy(asc(pokedexes.name));

  // 2. Fetch entries for all pokedexes
  const rawEntries = await db.select({
    pokedexId: pokedexEntries.pokedexId,
    pokemonId: pokedexEntries.pokemonId,
    entryNumber: pokedexEntries.entryNumber,
    nextPokemonId: pokedexEntries.nextPokemonId,
  })
  .from(pokedexEntries);

  const pokedexEntriesMap: Record<number, number[]> = {};

  for (const dex of pokedexesList) {
    const entries = rawEntries.filter(e => e.pokedexId === dex.id);
    if (entries.length === 0) {
      pokedexEntriesMap[dex.id] = [];
      continue;
    }

    let sorted = [];
    if (dex.source === 'custom') {
      sorted = sortLinkedListEntries(entries);
    } else {
      sorted = [...entries].sort((a, b) => a.entryNumber - b.entryNumber);
    }

    pokedexEntriesMap[dex.id] = sorted.map(s => s.pokemonId);
  }

  return { pokemon: rows, species: speciesList, typeRows, total, pokedexes: pokedexesList, pokedexEntriesMap };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = CreateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { createError: parsed.error.issues[0]?.message ?? 'Inválido', createValues: form });
    }
    const data = parsed.data;

    const [slugClash] = await db.select({ id: pokemon.id })
      .from(pokemon).where(eq(pokemon.slug, data.slug)).limit(1);
    if (slugClash) return fail(400, { createError: 'slug já em uso', createValues: form });

    const [nameClash] = await db.select({ id: pokemon.id })
      .from(pokemon).where(eq(pokemon.name, data.name)).limit(1);
    if (nameClash) return fail(400, { createError: 'name já em uso', createValues: form });

    const [species] = await db.select({ id: pokemonSpecies.id })
      .from(pokemonSpecies).where(eq(pokemonSpecies.id, data.speciesId)).limit(1);
    if (!species) return fail(400, { createError: 'speciesId inválido', createValues: form });

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
      spriteDefault: null,
      spriteShiny:   null,
      spriteArtwork: null,
      source:        'custom' as const,
      pokeapiId:     null,
    };

    // Look up the existing Base Form for this species (if any) to copy from.
    const [speciesRecord] = await db.select({ defaultPokemonId: pokemonSpecies.defaultPokemonId })
      .from(pokemonSpecies)
      .where(eq(pokemonSpecies.id, data.speciesId))
      .limit(1);

    const [baseForm] = speciesRecord && speciesRecord.defaultPokemonId != null
      ? await db.select({ id: pokemon.id })
          .from(pokemon)
          .where(eq(pokemon.id, speciesRecord.defaultPokemonId))
          .limit(1)
      : [];

    let baseTypes: { typeId: number; slot: number }[] = [];
    let baseAbilities: { abilityId: number; slot: number; isHidden: boolean }[] = [];
    let baseLearnset: { moveId: number; learnMethodId: number; level: number | null; versionGroupId: number | null }[] = [];

    if (baseForm) {
      baseTypes = await db.select({ typeId: pokemonTypes.typeId, slot: pokemonTypes.slot })
        .from(pokemonTypes).where(eq(pokemonTypes.pokemonId, baseForm.id));
      baseAbilities = await db.select({
        abilityId: pokemonAbilities.abilityId,
        slot:      pokemonAbilities.slot,
        isHidden:  pokemonAbilities.isHidden,
      }).from(pokemonAbilities).where(eq(pokemonAbilities.pokemonId, baseForm.id));
      baseLearnset = await db.select({
        moveId:         pokemonMoves.moveId,
        learnMethodId:  pokemonMoves.learnMethodId,
        level:          pokemonMoves.level,
        versionGroupId: pokemonMoves.versionGroupId,
      }).from(pokemonMoves).where(eq(pokemonMoves.pokemonId, baseForm.id));
    }

    // Transactionally insert the new pokemon + copy Base Form sub-tables + auto-create pokemon_forms
    const inserted = db.transaction((tx) => {
      const [row] = tx.insert(pokemon).values(newRow).returning({ id: pokemon.id }).all();

      if (baseTypes.length > 0) {
        tx.insert(pokemonTypes).values(baseTypes.map(t => ({
          pokemonId: row.id, typeId: t.typeId, slot: t.slot,
        }))).run();
      }
      if (baseAbilities.length > 0) {
        tx.insert(pokemonAbilities).values(baseAbilities.map(a => ({
          pokemonId: row.id, abilityId: a.abilityId, slot: a.slot, isHidden: a.isHidden,
        }))).run();
      }
      const validLearnset = baseLearnset.filter(m => m.versionGroupId !== null);
      if (validLearnset.length > 0) {
        tx.insert(pokemonMoves).values(validLearnset.map(m => ({
          pokemonId:      row.id,
          moveId:         m.moveId,
          learnMethodId:  m.learnMethodId,
          level:          m.level ?? undefined,
          versionGroupId: m.versionGroupId!,
        }))).run();
      }

      tx.insert(pokemonForms).values({
        pokemonId:     row.id,
        speciesId:     data.speciesId,
        formName:      null,
        isDefault:     true,
        isBattleOnly:  false,
        isMega:        false,
        order:         null,
        spriteDefault: null,
        spriteShiny:   null,
        spriteArtwork: null,
        source:        'custom',
        pokeapiId:     null,
      }).run();

      return row.id;
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon', inserted, null, {
        ...newRow,
        copiedFromBaseForm: baseForm?.id ?? null,
        copiedCounts: {
          types:     baseTypes.length,
          abilities: baseAbilities.length,
          learnset:  baseLearnset.length,
        },
      });
    }

    throw redirect(303, `/editor/pokemon/${data.slug}`);
  },
};
