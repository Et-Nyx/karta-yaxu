// @ts-nocheck
import { db } from '$lib/db';
import { desc, count, eq } from 'drizzle-orm';
import * as schema from '$lib/db/schema';
import { 
  importAllGameData, 
} from '$lib/pokeapi/import-service';
import { 
  importAllTypes,
  importAllAbilities,
  importAllItems,
  importAllMoves,
  importPokemonById,
  importPokemonByRange
} from '$lib/pokeapi/import-pokemon';
import type { Actions } from './$types';

// Helper for status count
async function getTableCounts(table: any) {
  const result = await db.select({
    source: table.source,
    c: count()
  }).from(table).groupBy(table.source);
  
  const counts = { total: 0, pokeapi: 0, edited: 0, custom: 0 };
  for (const row of result) {
    if (row.source === 'pokeapi') counts.pokeapi = row.c;
    else if (row.source === 'edited') counts.edited = row.c;
    else if (row.source === 'custom') counts.custom = row.c;
    counts.total += row.c;
  }
  return counts;
}

export const load = async () => {
  // Query status counts
  const status = {
    languages: await getTableCounts(schema.languages),
    regions: await getTableCounts(schema.regions),
    generations: await getTableCounts(schema.generations),
    types: await getTableCounts(schema.types),
    abilities: await getTableCounts(schema.abilities),
    moves: await getTableCounts(schema.moves),
    items: await getTableCounts(schema.items),
    pokemon: await getTableCounts(schema.pokemon),
  };

  // Query latest 50 logs
  const logs = await db.select()
    .from(schema.importLog)
    .orderBy(desc(schema.importLog.timestamp))
    .limit(50);

  return {
    status,
    logs
  };
};

export const actions = {
  importGameData: async () => {
    try {
      const res = await importAllGameData();
      return { success: true, message: `Imported Game Data (Languages: ${res.languages}, Generations: ${res.generations})` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importTypes: async () => {
    try {
      await importAllTypes();
      return { success: true, message: 'Imported Types' };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importAbilities: async () => {
    try {
      await importAllAbilities();
      return { success: true, message: 'Imported Abilities' };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importMoves: async () => {
    try {
      await importAllMoves();
      return { success: true, message: 'Imported Moves' };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importItems: async () => {
    try {
      await importAllItems();
      return { success: true, message: 'Imported Items' };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importSinglePokemon: async ({ request }: import('./$types').RequestEvent) => {
    const data = await request.formData();
    const id = parseInt(data.get('pokeapiId') as string);
    if (isNaN(id) || id <= 0) return { success: false, error: 'Invalid ID' };
    
    try {
      const resultId = await importPokemonById(id);
      return { success: true, message: `Imported Pokemon DB ID: ${resultId}` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  },
  importRangePokemon: async ({ request }: import('./$types').RequestEvent) => {
    const data = await request.formData();
    const fromId = parseInt(data.get('fromId') as string);
    const toId = parseInt(data.get('toId') as string);
    
    if (isNaN(fromId) || isNaN(toId) || fromId > toId || fromId <= 0) {
      return { success: false, error: 'Invalid Range' };
    }
    
    try {
      const res = await importPokemonByRange(fromId, toId);
      return { success: true, message: `Imported ${res.imported} Pokemon (${res.skipped} skipped)` };
    } catch (e: any) {
      return { success: false, error: e.message };
    }
  }
};
;null as any as Actions;