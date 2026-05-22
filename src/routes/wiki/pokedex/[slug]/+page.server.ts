import { db } from '$lib/db';
import { eq, asc, inArray, and } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  const { slug } = params;

  const pdexList = await db.select({
    id:           schema.pokedexes.id,
    name:         schema.pokedexes.name,
    slug:         schema.pokedexes.slug,
    isMainSeries: schema.pokedexes.isMainSeries,
    regionName:   schema.regions.name,
  })
  .from(schema.pokedexes)
  .leftJoin(schema.regions, eq(schema.pokedexes.regionId, schema.regions.id))
  .where(eq(schema.pokedexes.slug, slug))
  .limit(1);

  if (pdexList.length === 0) throw error(404, 'Pokédex not found');
  const pokedex = pdexList[0];

  const entries = await db.select({
    entryNumber:  schema.pokedexEntries.entryNumber,
    speciesName:  schema.pokemonSpecies.name,
    speciesSlug:  schema.pokemonSpecies.slug,
    pokemonId:    schema.pokemon.id,
    pokemonSlug:  schema.pokemon.slug,
    pokemonSprite: schema.pokemon.spriteDefault,
  })
  .from(schema.pokedexEntries)
  .innerJoin(schema.pokemonSpecies, eq(schema.pokedexEntries.speciesId, schema.pokemonSpecies.id))
  .innerJoin(schema.pokemon, eq(schema.pokemon.id, schema.pokemonSpecies.defaultPokemonId))
  .where(eq(schema.pokedexEntries.pokedexId, pokedex.id))
  .orderBy(asc(schema.pokedexEntries.entryNumber));

  // Fetch types for each pokemon
  const pokemonIds = entries.map(e => e.pokemonId);
  let typesByPokemon = new Map<number, Array<{ name: string; hexColor: string | null }>>();

  if (pokemonIds.length > 0) {
    const typeRows = await db.select({
      pokemonId:    schema.pokemonTypes.pokemonId,
      typeName:     schema.types.name,
      typeHexColor: schema.types.hexColor,
      slot:         schema.pokemonTypes.slot,
    })
    .from(schema.pokemonTypes)
    .innerJoin(schema.types, eq(schema.pokemonTypes.typeId, schema.types.id))
    .where(inArray(schema.pokemonTypes.pokemonId, pokemonIds))
    .orderBy(asc(schema.pokemonTypes.slot));

    for (const row of typeRows) {
      if (!typesByPokemon.has(row.pokemonId)) typesByPokemon.set(row.pokemonId, []);
      typesByPokemon.get(row.pokemonId)!.push({ name: row.typeName, hexColor: row.typeHexColor });
    }
  }

  const enrichedEntries = entries.map(e => ({
    entryNumber:  e.entryNumber,
    name:         e.speciesName,
    slug:         e.pokemonSlug,
    spriteUrl:    e.pokemonSprite,
    types:        typesByPokemon.get(e.pokemonId) ?? [],
  }));

  return { pokedex, entries: enrichedEntries };
};
