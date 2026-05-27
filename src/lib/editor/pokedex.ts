import { db } from '$lib/db';
import { and, eq, inArray } from 'drizzle-orm';
import { pokedexes } from '$lib/db/schema/game-metadata';
import { pokedexEntries, pokemon, pokemonSpecies } from '$lib/db/schema/pokemon';
import {
  getEvolutionChainSpecies,
  sortSpeciesByEvolutionRelation,
  getEvolutionChainPokemon,
  sortPokemonByEvolutionRelation
} from './evolution';

/**
 * Sorts flat database entries of a custom Pokédex by traversing its linked list structure.
 * Robustly self-heals in case of cyclical references or detached orphaned entries.
 */
export function sortLinkedListEntries<T extends { pokemonId: number; nextPokemonId: number | null }>(entries: T[]): T[] {
  if (entries.length <= 1) return entries;
  
  const map = new Map(entries.map(e => [e.pokemonId, e]));
  const pointedTo = new Set(entries.map(e => e.nextPokemonId).filter((id): id is number => id != null));
  
  // Find head pokemon: an entry in this list that no other entry in this list points to
  let head = entries.find(e => !pointedTo.has(e.pokemonId));
  
  // Fallback in case of perfect cycle or disconnected list: pick the first one
  if (!head) head = entries[0];
  
  const sorted: T[] = [];
  const visited = new Set<number>();
  let current: T | undefined = head;
  
  while (current && !visited.has(current.pokemonId)) {
    visited.add(current.pokemonId);
    sorted.push(current);
    current = current.nextPokemonId ? map.get(current.nextPokemonId) : undefined;
  }
  
  // Self-healing: if any entries were bypassed (disconnected nodes), append them
  if (sorted.length < entries.length) {
    for (const entry of entries) {
      if (!visited.has(entry.pokemonId)) {
        sorted.push(entry);
      }
    }
  }
  
  return sorted;
}

/**
 * Saves a new ordered list of Pokémon IDs for a custom Pokédex by rebuilding
 * the linked-list nextPokemonId pointers and recalculating regional Dex numbers.
 */
export async function savePokedexOrder(pokedexId: number, orderedPokemonIds: number[]): Promise<void> {
  if (orderedPokemonIds.length === 0) {
    db.transaction((tx) => {
      tx.delete(pokedexEntries).where(eq(pokedexEntries.pokedexId, pokedexId)).run();
    });
    return;
  }

  const pokemonRows = await db.select({ id: pokemon.id, speciesId: pokemon.speciesId })
    .from(pokemon)
    .where(inArray(pokemon.id, orderedPokemonIds));
  const pokemonToSpeciesMap = new Map(pokemonRows.map(p => [p.id, p.speciesId]));

  db.transaction((tx) => {
    tx.delete(pokedexEntries).where(eq(pokedexEntries.pokedexId, pokedexId)).run();

    for (let i = 0; i < orderedPokemonIds.length; i++) {
      const pokemonId = orderedPokemonIds[i];
      const nextPokemonId = i < orderedPokemonIds.length - 1 ? orderedPokemonIds[i + 1] : null;
      const speciesId = pokemonToSpeciesMap.get(pokemonId);
      if (speciesId == null) continue;

      const nextSpeciesId = nextPokemonId ? pokemonToSpeciesMap.get(nextPokemonId) ?? null : null;
      const entryNumber = i + 1;

      tx.insert(pokedexEntries)
        .values({
          pokedexId,
          pokemonId,
          speciesId,
          entryNumber,
          nextPokemonId,
          nextSpeciesId
        })
        .run();
    }
  });
}

/**
 * Adds a Pokémon form to a custom Pokédex, automatically fetching, sorting, and
 * appending its entire form-based evolution chain to the end of the list.
 */
export async function addPokemonToPokedex(pokedexId: number, pokemonId: number): Promise<void> {
  // 1. Fetch entire form-based evolution chain pokemon IDs
  const chainPokemonIds = await getEvolutionChainPokemon(pokemonId);
  if (chainPokemonIds.length === 0) return;

  // 2. Fetch existing entries
  const existing = await db.select({ pokemonId: pokedexEntries.pokemonId })
    .from(pokedexEntries)
    .where(eq(pokedexEntries.pokedexId, pokedexId));
  const existingSet = new Set(existing.map(e => e.pokemonId));

  // Filter to only non-existing ones
  const newPokemonIds = chainPokemonIds.filter(pId => !existingSet.has(pId));
  if (newPokemonIds.length === 0) return;

  // Fetch species IDs for these new pokemon
  const pokemonRows = await db.select({ id: pokemon.id, speciesId: pokemon.speciesId })
    .from(pokemon)
    .where(inArray(pokemon.id, newPokemonIds));
  const pokemonToSpeciesMap = new Map(pokemonRows.map(p => [p.id, p.speciesId]));

  // 3. Update pointers and append new entries inside a transaction
  db.transaction((tx) => {
    const currentRows = tx.select({
      pokemonId: pokedexEntries.pokemonId,
      speciesId: pokedexEntries.speciesId,
      nextPokemonId: pokedexEntries.nextPokemonId,
      nextSpeciesId: pokedexEntries.nextSpeciesId,
      entryNumber: pokedexEntries.entryNumber
    })
      .from(pokedexEntries)
      .where(eq(pokedexEntries.pokedexId, pokedexId))
      .all();

    const sortedCurrent = sortLinkedListEntries(currentRows);
    const lastEntry = sortedCurrent[sortedCurrent.length - 1];
    let currentNumber = sortedCurrent.length;

    // Link the previous tail to the first new element
    if (lastEntry) {
      tx.update(pokedexEntries)
        .set({ 
          nextPokemonId: newPokemonIds[0],
          nextSpeciesId: pokemonToSpeciesMap.get(newPokemonIds[0]) ?? null
        })
        .where(and(
          eq(pokedexEntries.pokedexId, pokedexId),
          eq(pokedexEntries.pokemonId, lastEntry.pokemonId)
        ))
        .run();
    }

    // Insert new entries linked sequentially
    for (let i = 0; i < newPokemonIds.length; i++) {
      const pId = newPokemonIds[i];
      const nextPId = i < newPokemonIds.length - 1 ? newPokemonIds[i + 1] : null;
      
      const sId = pokemonToSpeciesMap.get(pId);
      if (sId == null) continue;
      const nextSId = nextPId ? pokemonToSpeciesMap.get(nextPId) ?? null : null;
      currentNumber++;

      tx.insert(pokedexEntries).values({
        pokedexId,
        pokemonId: pId,
        speciesId: sId,
        entryNumber: currentNumber,
        nextPokemonId: nextPId,
        nextSpeciesId: nextSId
      }).run();
    }
  });
}

/**
 * Removes a Pokémon form from a custom Pokédex, automatically removing all
 * related forms in its evolution chain and re-linking the remaining entries.
 */
export async function removePokemonFromPokedex(pokedexId: number, pokemonId: number): Promise<void> {
  const chainPokemonIds = await getEvolutionChainPokemon(pokemonId);
  if (chainPokemonIds.length === 0) return;
  const toRemoveSet = new Set(chainPokemonIds);

  db.transaction((tx) => {
    const currentRows = tx.select({
      pokemonId: pokedexEntries.pokemonId,
      speciesId: pokedexEntries.speciesId,
      nextPokemonId: pokedexEntries.nextPokemonId,
      nextSpeciesId: pokedexEntries.nextSpeciesId,
      entryNumber: pokedexEntries.entryNumber
    })
      .from(pokedexEntries)
      .where(eq(pokedexEntries.pokedexId, pokedexId))
      .all();

    const sortedCurrent = sortLinkedListEntries(currentRows);
    
    // Filter remaining entries
    const remaining = sortedCurrent.filter(e => !toRemoveSet.has(e.pokemonId));

    // Re-link remaining entries and update regional dex numbers
    for (let i = 0; i < remaining.length; i++) {
      const entry = remaining[i];
      const nextPId = i < remaining.length - 1 ? remaining[i + 1].pokemonId : null;
      const nextSId = i < remaining.length - 1 ? remaining[i + 1].speciesId : null;
      const entryNumber = i + 1;

      tx.update(pokedexEntries)
        .set({ nextPokemonId: nextPId, nextSpeciesId: nextSId, entryNumber })
        .where(and(
          eq(pokedexEntries.pokedexId, pokedexId),
          eq(pokedexEntries.pokemonId, entry.pokemonId)
        ))
        .run();
    }

    // Delete removed entries from database
    tx.delete(pokedexEntries)
      .where(and(
        eq(pokedexEntries.pokedexId, pokedexId),
        inArray(pokedexEntries.pokemonId, chainPokemonIds)
      ))
      .run();
  });
}

/**
 * Automatically synchronizes a custom Pokédex with any recent evolution chain changes
 * (e.g. newly created forms or updated evolution relations).
 */
export async function syncCustomPokedexEntries(pokedexId: number): Promise<void> {
  // 1. Fetch current database entries
  const currentRows = await db.select({
    pokemonId: pokedexEntries.pokemonId,
    speciesId: pokedexEntries.speciesId,
    nextPokemonId: pokedexEntries.nextPokemonId,
    nextSpeciesId: pokedexEntries.nextSpeciesId,
    entryNumber: pokedexEntries.entryNumber
  })
    .from(pokedexEntries)
    .where(eq(pokedexEntries.pokedexId, pokedexId));

  if (currentRows.length === 0) return;

  const sortedCurrent = sortLinkedListEntries(currentRows);
  const currentPokemonIds = sortedCurrent.map(e => e.pokemonId);
  const currentSet = new Set(currentPokemonIds);

  // 2. Resolve all unique evolution chain groups present in this pokedex
  const processedSet = new Set<number>();
  const missingPokemonToAdd: Array<{ insertAfterId: number; idsToAdd: number[] }> = [];

  for (const entry of sortedCurrent) {
    if (processedSet.has(entry.pokemonId)) continue;

    // Get full form-based evolution chain for this pokemon
    const chain = await getEvolutionChainPokemon(entry.pokemonId);
    
    // Mark all members of this chain as processed so we don't scan them again
    for (const id of chain) {
      processedSet.add(id);
    }

    // Check if there are any missing pokemon forms in the database chain
    const missingInDex = chain.filter(id => !currentSet.has(id));
    if (missingInDex.length > 0) {
      // We want to insert the missing forms right after the last existing member of this chain!
      let lastExistingIndex = -1;
      for (let i = sortedCurrent.length - 1; i >= 0; i--) {
        if (chain.includes(sortedCurrent[i].pokemonId)) {
          lastExistingIndex = i;
          break;
        }
      }

      if (lastExistingIndex !== -1) {
        const lastExistingId = sortedCurrent[lastExistingIndex].pokemonId;
        missingPokemonToAdd.push({
          insertAfterId: lastExistingId,
          idsToAdd: missingInDex
        });
      }
    }
  }

  // If no missing forms to add, we're fully in sync!
  if (missingPokemonToAdd.length === 0) return;

  // 3. Build the new synchronized ordered list of pokemon IDs
  let newOrder: number[] = [...currentPokemonIds];

  for (const { insertAfterId, idsToAdd } of missingPokemonToAdd) {
    const index = newOrder.indexOf(insertAfterId);
    if (index !== -1) {
      // Insert all missing forms contiguously right after the last sibling
      newOrder.splice(index + 1, 0, ...idsToAdd);
    }
  }

  // 4. Atomically persist the new sequence back to database
  await savePokedexOrder(pokedexId, newOrder);
}

/**
 * Copies a static, read-only Pokédex to create a brand new custom Pokédex.
 * Automatically resolves and maps all species entries to their FULL evolution chains,
 * resolved to base/default Pokémon form IDs, ensuring total custom editor stability.
 */
export async function copyStaticPokedex(
  staticPokedexId: number,
  name: string,
  slug: string,
  regionId: number | null
): Promise<number> {
  // 1. Fetch static entries first (asynchronously)
  const staticEntries = await db.select({
    pokemonId: pokedexEntries.pokemonId,
    speciesId: pokedexEntries.speciesId,
    entryNumber: pokedexEntries.entryNumber
  })
    .from(pokedexEntries)
    .where(eq(pokedexEntries.pokedexId, staticPokedexId))
    .orderBy(pokedexEntries.entryNumber);

  if (staticEntries.length === 0) {
    return db.transaction((tx) => {
      const [newPokedex] = tx.insert(pokedexes).values({
        name,
        slug,
        regionId,
        isMainSeries: false,
        source: 'custom',
        pokeapiId: null
      }).returning({ id: pokedexes.id }).all();
      return newPokedex.id;
    });
  }

  // 2. Expand each species to its entire evolution chain, preserving Pokédex order
  const speciesIdsOrdered: number[] = [];
  const seenSpecies = new Set<number>();

  for (const entry of staticEntries) {
    if (seenSpecies.has(entry.speciesId)) {
      continue;
    }

    const chainSpeciesIds = await getEvolutionChainSpecies(entry.speciesId);
    const sortedChain = await sortSpeciesByEvolutionRelation(chainSpeciesIds);

    for (const sId of sortedChain) {
      if (!seenSpecies.has(sId)) {
        seenSpecies.add(sId);
        speciesIdsOrdered.push(sId);
      }
    }
  }

  // 3. Map each speciesId in the expanded chain to its default/base pokemonId (asynchronously)
  const speciesToPokemonMap = new Map<number, number>();
  
  if (speciesIdsOrdered.length > 0) {
    const speciesRows = await db.select({
      id: pokemonSpecies.id,
      defaultPokemonId: pokemonSpecies.defaultPokemonId
    })
      .from(pokemonSpecies)
      .where(inArray(pokemonSpecies.id, speciesIdsOrdered));

    for (const s of speciesRows) {
      if (s.defaultPokemonId) {
        speciesToPokemonMap.set(s.id, s.defaultPokemonId);
      }
    }

    const missingSpeciesIds = speciesIdsOrdered.filter(sId => !speciesToPokemonMap.has(sId));
    if (missingSpeciesIds.length > 0) {
      const fallbackRows = await db.select({
        id: pokemon.id,
        speciesId: pokemon.speciesId
      })
        .from(pokemon)
        .where(inArray(pokemon.speciesId, missingSpeciesIds));

      for (const f of fallbackRows) {
        if (!speciesToPokemonMap.has(f.speciesId)) {
          speciesToPokemonMap.set(f.speciesId, f.id);
        }
      }
    }
  }

  // 4. Map the fully expanded, ordered list of species to form entries
  const mappedEntries = speciesIdsOrdered.map(speciesId => {
    const resolvedPokemonId = speciesToPokemonMap.get(speciesId);
    return {
      speciesId,
      pokemonId: resolvedPokemonId ?? speciesId // Fallback safety
    };
  });

  // 5. Perform the final atomic database write synchronously inside the transaction block
  return db.transaction((tx) => {
    const [newPokedex] = tx.insert(pokedexes).values({
      name,
      slug,
      regionId,
      isMainSeries: false,
      source: 'custom',
      pokeapiId: null
    }).returning({ id: pokedexes.id }).all();

    for (let i = 0; i < mappedEntries.length; i++) {
      const entry = mappedEntries[i];
      const nextPokemonId = i < mappedEntries.length - 1 ? mappedEntries[i + 1].pokemonId : null;
      const nextSpeciesId = i < mappedEntries.length - 1 ? mappedEntries[i + 1].speciesId : null;

      tx.insert(pokedexEntries).values({
        pokedexId: newPokedex.id,
        pokemonId: entry.pokemonId,
        speciesId: entry.speciesId,
        entryNumber: i + 1,
        nextPokemonId,
        nextSpeciesId
      }).run();
    }

    return newPokedex.id;
  });
}
