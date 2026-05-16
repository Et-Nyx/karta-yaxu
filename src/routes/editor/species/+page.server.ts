import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq } from 'drizzle-orm';
import { z } from 'zod';
import { db } from '$lib/db';
import { pokemon, pokemonForms, pokemonSpecies } from '$lib/db/schema/pokemon';
import { generations } from '$lib/db/schema/game-metadata';
import { recordEdit } from '$lib/editor/audit';
import type { PageServerLoad, Actions } from './$types';

const slugRegex = /^[a-z0-9-]+$/;

const boolField = z.preprocess(
  (v) => v === 'true' || v === 'on' || v === true,
  z.boolean(),
);

const intOrNull = z.preprocess(
  (v) => (v === '' || v === null || v === undefined ? null : Number(v)),
  z.number().int().nullable(),
);

const CreateSchema = z.object({
  name:        z.string().trim().min(1).max(80),
  slug:        z.string().trim().min(1).max(80).regex(slugRegex, 'slug: lowercase letras/números/hífen'),
  dexNumber:   z.coerce.number().int().min(0).max(9999),
  genus:       z.preprocess((v) => v === '' ? null : v, z.string().nullable()),
  isBaby:      boolField,
  isLegendary: boolField,
  isMythical:  boolField,
});

export const load: PageServerLoad = async () => {
  const rows = await db.select({
    id:           pokemonSpecies.id,
    name:         pokemonSpecies.name,
    slug:         pokemonSpecies.slug,
    genus:        pokemonSpecies.genus,
    dexNumber:    pokemonSpecies.dexNumber,
    isBaby:       pokemonSpecies.isBaby,
    isLegendary:  pokemonSpecies.isLegendary,
    isMythical:   pokemonSpecies.isMythical,
    generationId: pokemonSpecies.generationId,
    source:       pokemonSpecies.source,
    pokeapiId:    pokemonSpecies.pokeapiId,
  }).from(pokemonSpecies).orderBy(asc(pokemonSpecies.dexNumber));

  const generationList = await db.select({
    id:   generations.id,
    name: generations.name,
    slug: generations.slug,
  }).from(generations).orderBy(asc(generations.id));

  const [{ total }] = await db.select({ total: count() }).from(pokemonSpecies);

  return { species: rows, generations: generationList, total };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = CreateSchema.safeParse(form);
    if (!parsed.success) {
      return fail(400, { createError: parsed.error.issues[0]?.message ?? 'Inválido', createValues: form });
    }
    const data = parsed.data;

    const [slugClash] = await db.select({ id: pokemonSpecies.id })
      .from(pokemonSpecies).where(eq(pokemonSpecies.slug, data.slug)).limit(1);
    if (slugClash) return fail(400, { createError: 'slug já em uso', createValues: form });

    const [pokemonSlugClash] = await db.select({ id: pokemon.id })
      .from(pokemon).where(eq(pokemon.slug, data.slug)).limit(1);
    if (pokemonSlugClash) return fail(400, { createError: 'slug já em uso por um pokemon', createValues: form });

    // Transactionally create Species + default Pokemon + default PokemonForm (the Base Form pair)
    const inserted = db.transaction((tx) => {
      const [newSpecies] = tx.insert(pokemonSpecies).values({
        name:         data.name,
        slug:         data.slug,
        dexNumber:    data.dexNumber,
        genus:        data.genus,
        isBaby:       data.isBaby,
        isLegendary:  data.isLegendary,
        isMythical:   data.isMythical,
        source:       'custom',
        pokeapiId:    null,
      }).returning({ id: pokemonSpecies.id }).all();

      const [newPokemon] = tx.insert(pokemon).values({
        name:          data.name,
        slug:          data.slug,
        speciesId:     newSpecies.id,
        height:        10,
        weight:        100,
        hp:            50,
        atk:           50,
        def:           50,
        spa:           50,
        spd:           50,
        spe:           50,
        spriteDefault: null,
        spriteShiny:   null,
        spriteArtwork: null,
        isDefault:     true,
        order:         null,
        source:        'custom',
        pokeapiId:     null,
      }).returning({ id: pokemon.id }).all();

      const [newForm] = tx.insert(pokemonForms).values({
        pokemonId:     newPokemon.id,
        speciesId:     newSpecies.id,
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
      }).returning({ id: pokemonForms.id }).all();

      return { speciesId: newSpecies.id, pokemonId: newPokemon.id, formId: newForm.id };
    });

    if (locals.user) {
      await recordEdit(locals.user.id, 'pokemon_species', inserted.speciesId, null, data);
      await recordEdit(locals.user.id, 'pokemon',         inserted.pokemonId, null, { autoCreated: 'base-form', speciesId: inserted.speciesId });
      await recordEdit(locals.user.id, 'pokemon_forms',   inserted.formId,    null, { autoCreated: 'base-form', pokemonId: inserted.pokemonId });
    }

    throw redirect(303, `/editor/species/${data.slug}`);
  },
};
