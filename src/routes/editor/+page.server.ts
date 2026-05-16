import { count, desc, eq } from 'drizzle-orm';
import { db } from '$lib/db';
import { types } from '$lib/db/schema/core-data';
import { abilities, moves } from '$lib/db/schema/core-data';
import { items, natures } from '$lib/db/schema/items';
import { pokemon, pokemonSpecies } from '$lib/db/schema/pokemon';
import { pokedexes } from '$lib/db/schema/game-metadata';
import { edits, trainers } from '$lib/db/schema/trainers';
import type { PageServerLoad } from './$types';

type CountRow = { source: string; n: number };

async function countsBySource(table: any): Promise<{ total: number; pokeapi: number; edited: number; custom: number }> {
  const rows = await db.select({ source: table.source, n: count() })
    .from(table)
    .groupBy(table.source) as CountRow[];

  const out = { total: 0, pokeapi: 0, edited: 0, custom: 0 };
  for (const r of rows) {
    out.total += r.n;
    if (r.source === 'pokeapi') out.pokeapi = r.n;
    else if (r.source === 'edited') out.edited = r.n;
    else if (r.source === 'custom') out.custom = r.n;
  }
  return out;
}

export const load: PageServerLoad = async () => {
  const [
    typesCounts,
    naturesCounts,
    abilitiesCounts,
    itemsCounts,
    pokemonCounts,
    speciesCounts,
    movesCounts,
    pokedexesCounts,
  ] = await Promise.all([
    countsBySource(types),
    countsBySource(natures),
    countsBySource(abilities),
    countsBySource(items),
    countsBySource(pokemon),
    countsBySource(pokemonSpecies),
    countsBySource(moves),
    countsBySource(pokedexes),
  ]);

  const recentEdits = await db.select({
    id:        edits.id,
    tableName: edits.tableName,
    recordId:  edits.recordId,
    editedAt:  edits.editedAt,
    username:  trainers.username,
  })
    .from(edits)
    .leftJoin(trainers, eq(edits.trainerId, trainers.id))
    .orderBy(desc(edits.editedAt))
    .limit(15);

  return {
    entities: [
      { key: 'types',     label: 'Types',      href: '/editor/types',     counts: typesCounts,     ready: true },
      { key: 'natures',   label: 'Natures',    href: '/editor/natures',   counts: naturesCounts,   ready: true },
      { key: 'abilities', label: 'Abilities',  href: '/editor/abilities', counts: abilitiesCounts, ready: true },
      { key: 'items',     label: 'Items',      href: '/editor/items',     counts: itemsCounts,     ready: true },
      { key: 'pokedexes', label: 'Pokédexes',  href: '/editor/pokedex',   counts: pokedexesCounts, ready: true },
      { key: 'moves',     label: 'Moves',      href: '/editor/moves',     counts: movesCounts,     ready: true },
      { key: 'pokemon',   label: 'Pokémon',    href: '/editor/pokemon',   counts: pokemonCounts,   ready: true },
      { key: 'species',   label: 'Species',    href: '/editor/species',   counts: speciesCounts,   ready: true },
    ],
    recentEdits,
  };
};
