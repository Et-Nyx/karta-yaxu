import { and, eq } from 'drizzle-orm';
import { db } from '$lib/db/index';
import * as schema from '$lib/db/schema/index';
import * as client from './client';
import { importEvents } from './import-events';

export async function triggerBackgroundStaging(
  trainerId: string,
  entityType: string,
  ids: number[]
) {
  // 1. Clear previous staged items for this trainer
  await db
    .delete(schema.stagedImports)
    .where(eq(schema.stagedImports.trainerId, trainerId));

  // 2. Insert pending staging rows
  const rows = ids.map((id) => ({
    trainerId,
    entityType,
    pokeapiId: id,
    status: 'pending' as const,
  }));
  await db.insert(schema.stagedImports).values(rows);

  // 3. Initiate non-blocking background fetching
  runBackgroundStaging(trainerId, entityType, ids).catch(console.error);
}

async function runBackgroundStaging(
  trainerId: string,
  entityType: string,
  ids: number[]
) {
  let fetchedCount = 0;

  for (const id of ids) {
    try {
      // Update status to fetching
      await db
        .update(schema.stagedImports)
        .set({ status: 'fetching', updatedAt: new Date() })
        .where(
          and(
            eq(schema.stagedImports.trainerId, trainerId),
            eq(schema.stagedImports.pokeapiId, id)
          )
        );

      let payloadObj: any = null;

      // Fetch according to type
      if (entityType === 'pokemon') {
        const species = await client.fetchPokemonSpecies(id);
        const pokemon = await client.fetchPokemon(id);
        payloadObj = { species, pokemon };
      } else if (entityType === 'abilities') {
        payloadObj = await client.fetchAbility(id);
      } else if (entityType === 'moves') {
        payloadObj = await client.fetchMove(id);
      } else if (entityType === 'items') {
        payloadObj = await client.fetchItem(id);
      } else if (entityType === 'types') {
        payloadObj = await client.fetchType(id);
      } else if (entityType === 'locations') {
        payloadObj = await client.fetchLocation(id);
      } else if (entityType === 'pokedexes') {
        payloadObj = await client.fetchPokedex(id);
      }

      // Save raw JSON payload
      await db
        .update(schema.stagedImports)
        .set({
          status: 'fetched',
          payload: JSON.stringify(payloadObj),
          updatedAt: new Date(),
        })
        .where(
          and(
            eq(schema.stagedImports.trainerId, trainerId),
            eq(schema.stagedImports.pokeapiId, id)
          )
        );

      fetchedCount++;

      // Emit staging SSE progress
      importEvents.emit('log', {
        entityType: 'staging',
        pokeapiId: id,
        action: 'inserted', // 'inserted' represents successfully fetched in the staging progress UI
        reason: `Staged ${fetchedCount}/${ids.length} ${entityType}`,
        comment: JSON.stringify({ trainerId, entityType, fetched: fetchedCount, total: ids.length }),
        timestamp: new Date().toISOString(),
      });
    } catch (e: any) {
      console.error(`Failed to stage ${entityType} #${id}:`, e);
      await db
        .update(schema.stagedImports)
        .set({
          status: 'failed',
          errorMessage: e.message,
          updatedAt: new Date(),
        })
        .where(
          and(
            eq(schema.stagedImports.trainerId, trainerId),
            eq(schema.stagedImports.pokeapiId, id)
          )
        );

      importEvents.emit('log', {
        entityType: 'staging',
        pokeapiId: id,
        action: 'skipped',
        reason: `Failed staging ${id}: ${e.message}`,
        comment: JSON.stringify({ trainerId, entityType, fetched: fetchedCount, total: ids.length, error: true }),
        timestamp: new Date().toISOString(),
      });
    }
  }
}

export async function getPokemonSpeciesIdsForRegion(regionPokeapiId: number): Promise<number[]> {
  const region = await client.fetchRegion(regionPokeapiId);
  if (!region.main_generation) throw new Error('Region has no main generation');
  const genId = client.extractIdFromUrl(region.main_generation.url);
  const gen = await client.fetchGeneration(genId);
  return gen.pokemon_species.map(ref => client.extractIdFromUrl(ref.url));
}
