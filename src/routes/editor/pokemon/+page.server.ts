import { fail, redirect } from '@sveltejs/kit';
import { and, asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import {
  pokemon,
  pokemonAbilities,
  pokemonForms,
  pokemonMoves,
  pokemonSpecies,
  pokemonTypes,
} from '$lib/db/schema/pokemon';
import { recordEdit } from '$lib/editor/audit';
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
    .orderBy(asc(pokemon.speciesId));

  const speciesList = await db.select({
    id:        pokemonSpecies.id,
    name:      pokemonSpecies.name,
    slug:      pokemonSpecies.slug,
    dexNumber: pokemonSpecies.dexNumber,
  }).from(pokemonSpecies).orderBy(asc(pokemonSpecies.dexNumber));

  const [{ total }] = await db.select({ total: count() }).from(pokemon);

  return { pokemon: rows, species: speciesList, total };
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
      isDefault:     false, // new variants are NOT the base form by default
      order:         null,
      source:        'custom' as const,
      pokeapiId:     null,
    };

    // Look up the existing Base Form for this species (if any) to copy from.
    const [baseForm] = await db.select({ id: pokemon.id })
      .from(pokemon)
      .where(and(eq(pokemon.speciesId, data.speciesId), eq(pokemon.isDefault, true)))
      .limit(1);

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
      if (baseLearnset.length > 0) {
        tx.insert(pokemonMoves).values(baseLearnset.map(m => ({
          pokemonId:      row.id,
          moveId:         m.moveId,
          learnMethodId:  m.learnMethodId,
          level:          m.level,
          versionGroupId: m.versionGroupId,
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
