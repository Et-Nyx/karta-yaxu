import { db } from '$lib/db';
import { and, eq, inArray } from 'drizzle-orm';
import { pokedexes } from '$lib/db/schema/game-metadata';
import { pokedexEntries, pokemon, pokemonSpecies } from '$lib/db/schema/pokemon';
import { getEvolutionChainSpecies, sortSpeciesByEvolutionRelation } from './evolution';

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
 * appending its entire evolution chain (in relation order) to the end of the list.
 */
export async function addPokemonToPokedex(pokedexId: number, pokemonId: number): Promise<void> {
  // 1. Fetch the pokemon form and species
  const [chosen] = await db.select({ speciesId: pokemon.speciesId })
    .from(pokemon)
    .where(eq(pokemon.id, pokemonId))
    .limit(1);
  if (!chosen) return;

  const chosenSpeciesId = chosen.speciesId;

  // 2. Fetch entire evolution chain species
  const chainSpeciesIds = await getEvolutionChainSpecies(chosenSpeciesId);

  // 3. Fetch existing entries
  const existing = await db.select({ pokemonId: pokedexEntries.pokemonId })
    .from(pokedexEntries)
    .where(eq(pokedexEntries.pokedexId, pokedexId));
  const existingSet = new Set(existing.map(e => e.pokemonId));

  // 4. Map each species in the chain to the correct pokemonId
  // (either the chosen pokemonId, or the species' defaultPokemonId)
  const speciesList = await db.select({ id: pokemonSpecies.id, defaultPokemonId: pokemonSpecies.defaultPokemonId })
    .from(pokemonSpecies)
    .where(inArray(pokemonSpecies.id, chainSpeciesIds));
  
  const speciesToPokemonMap = new Map<number, number>();
  for (const s of speciesList) {
    if (s.id === chosenSpeciesId) {
      speciesToPokemonMap.set(s.id, pokemonId);
    } else if (s.defaultPokemonId) {
      speciesToPokemonMap.set(s.id, s.defaultPokemonId);
    }
  }

  // Filter to only non-existing ones
  const newPokemonIds = chainSpeciesIds
    .map(sId => speciesToPokemonMap.get(sId)!)
    .filter(pId => pId && !existingSet.has(pId));
  
  if (newPokemonIds.length === 0) return;

  // Sort the new pokemon IDs by their species' evolution relation order
  const orderedNewSpeciesIds = await sortSpeciesByEvolutionRelation(
    chainSpeciesIds.filter(sId => newPokemonIds.includes(speciesToPokemonMap.get(sId)!))
  );

  const orderedNewPokemonIds = orderedNewSpeciesIds.map(sId => speciesToPokemonMap.get(sId)!);

  // 5. Update pointers and append new entries inside a transaction
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
          nextPokemonId: orderedNewPokemonIds[0],
          nextSpeciesId: orderedNewSpeciesIds[0]
        })
        .where(and(
          eq(pokedexEntries.pokedexId, pokedexId),
          eq(pokedexEntries.pokemonId, lastEntry.pokemonId)
        ))
        .run();
    }

    // Insert new entries linked sequentially
    for (let i = 0; i < orderedNewPokemonIds.length; i++) {
      const pId = orderedNewPokemonIds[i];
      const nextPId = i < orderedNewPokemonIds.length - 1 ? orderedNewPokemonIds[i + 1] : null;
      
      const sId = orderedNewSpeciesIds[i];
      const nextSId = i < orderedNewSpeciesIds.length - 1 ? orderedNewSpeciesIds[i + 1] : null;
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
 * related species in its evolution chain and re-linking the remaining entries.
 */
export async function removePokemonFromPokedex(pokedexId: number, pokemonId: number): Promise<void> {
  const [chosen] = await db.select({ speciesId: pokemon.speciesId })
    .from(pokemon)
    .where(eq(pokemon.id, pokemonId))
    .limit(1);
  if (!chosen) return;

  const chainIds = await getEvolutionChainSpecies(chosen.speciesId);
  const toRemoveSpeciesSet = new Set(chainIds);

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
    const remaining = sortedCurrent.filter(e => !toRemoveSpeciesSet.has(e.speciesId));

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
    if (chainIds.length > 0) {
      tx.delete(pokedexEntries)
        .where(and(
          eq(pokedexEntries.pokedexId, pokedexId),
          inArray(pokedexEntries.speciesId, chainIds)
        ))
        .run();
    }
  });
}

/**
 * Copies a static, read-only Pokédex to create a brand new custom Pokédex.
 * Links all entries sequentially using the linked-list pattern in the database.
 */
export async function copyStaticPokedex(
  staticPokedexId: number,
  name: string,
  slug: string,
  regionId: number | null
): Promise<number> {
  return db.transaction((tx) => {
    const [newPokedex] = tx.insert(pokedexes).values({
      name,
      slug,
      regionId,
      isMainSeries: false,
      source: 'custom',
      pokeapiId: null
    }).returning({ id: pokedexes.id }).all();

    const staticEntries = tx.select({
      pokemonId: pokedexEntries.pokemonId,
      speciesId: pokedexEntries.speciesId,
      entryNumber: pokedexEntries.entryNumber
    })
      .from(pokedexEntries)
      .where(eq(pokedexEntries.pokedexId, staticPokedexId))
      .orderBy(pokedexEntries.entryNumber)
      .all();

    for (let i = 0; i < staticEntries.length; i++) {
      const entry = staticEntries[i];
      const nextPokemonId = i < staticEntries.length - 1 ? staticEntries[i + 1].pokemonId : null;
      const nextSpeciesId = i < staticEntries.length - 1 ? staticEntries[i + 1].speciesId : null;

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
