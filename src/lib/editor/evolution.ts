import { db } from '$lib/db';
import { eq, inArray } from 'drizzle-orm';
import { pokemon } from '$lib/db/schema/pokemon';

/**
 * Fetches all unique species IDs within the evolution chain of a given species ID.
 * Walks up to the root parents and then BFS-traverses down to get all descendants.
 */
export async function getEvolutionChainSpecies(speciesId: number): Promise<number[]> {
  // 1. Fetch all pokemon variants for this species
  const speciesPokemon = await db.select({ id: pokemon.id, evolvesFromPokemonId: pokemon.evolvesFromPokemonId })
    .from(pokemon)
    .where(eq(pokemon.speciesId, speciesId));

  if (speciesPokemon.length === 0) {
    return [speciesId];
  }

  // 2. For each variant, walk up to its root parent pokemon
  const roots = new Set<number>();
  for (const pk of speciesPokemon) {
    let rootId = pk.id;
    const seen = new Set<number>([rootId]);
    let cursor = pk.evolvesFromPokemonId;
    while (cursor != null && !seen.has(cursor)) {
      seen.add(cursor);
      rootId = cursor;
      const [parent] = await db.select({ p: pokemon.evolvesFromPokemonId })
        .from(pokemon).where(eq(pokemon.id, cursor)).limit(1);
      cursor = parent?.p ?? null;
    }
    roots.add(rootId);
  }

  // 3. BFS down from all roots to collect all descendant and related pokemon variants
  const allPokemonIds = new Set<number>([...roots]);
  let frontier = [...roots];

  while (frontier.length > 0) {
    const children = await db.select({ id: pokemon.id })
      .from(pokemon)
      .where(inArray(pokemon.evolvesFromPokemonId, frontier));
    
    const nextFrontier: number[] = [];
    for (const c of children) {
      if (!allPokemonIds.has(c.id)) {
        allPokemonIds.add(c.id);
        nextFrontier.push(c.id);
      }
    }
    frontier = nextFrontier;
  }

  // 4. Retrieve unique species IDs for all collected pokemon variants
  const rows = await db.select({ speciesId: pokemon.speciesId })
    .from(pokemon)
    .where(inArray(pokemon.id, [...allPokemonIds]));

  const uniqueSpeciesIds = [...new Set(rows.map(r => r.speciesId))];
  return uniqueSpeciesIds.sort((a, b) => a - b);
}

/**
 * Sorts a list of species IDs based on their evolution chain relationships.
 * Base/parent forms are positioned before their evolved/child forms.
 */
export async function sortSpeciesByEvolutionRelation(speciesIds: number[]): Promise<number[]> {
  if (speciesIds.length <= 1) return speciesIds;

  // 1. Fetch all pokemon variants for these species
  const pList = await db.select({
    id: pokemon.id,
    speciesId: pokemon.speciesId,
    evolvesFromPokemonId: pokemon.evolvesFromPokemonId
  })
    .from(pokemon)
    .where(inArray(pokemon.speciesId, speciesIds));

  // Create a map from pokemon ID to its record
  const pokemonMap = new Map(pList.map(p => [p.id, p]));

  // Build the adjacency list: parent -> children
  const adj = new Map<number, number[]>();
  const isChild = new Set<number>();

  for (const p of pList) {
    if (p.evolvesFromPokemonId && pokemonMap.has(p.evolvesFromPokemonId)) {
      isChild.add(p.id);
      if (!adj.has(p.evolvesFromPokemonId)) {
        adj.set(p.evolvesFromPokemonId, []);
      }
      adj.get(p.evolvesFromPokemonId)!.push(p.id);
    }
  }

  // Find the roots: Pokemon that have no parent in our set
  const roots = pList.filter(p => !p.evolvesFromPokemonId || !pokemonMap.has(p.evolvesFromPokemonId));

  // Traverse using BFS/DFS to get a topological order of Pokemon
  const orderedPokemonIds: number[] = [];
  const visited = new Set<number>();

  function traverse(id: number) {
    if (visited.has(id)) return;
    visited.add(id);
    orderedPokemonIds.push(id);
    const children = adj.get(id) ?? [];
    for (const childId of children) {
      traverse(childId);
    }
  }

  // Traverse from each root
  for (const root of roots) {
    traverse(root.id);
  }

  // Map the ordered Pokemon back to their unique species IDs in the visited order
  const orderedSpeciesIds: number[] = [];
  const seenSpecies = new Set<number>();
  for (const pId of orderedPokemonIds) {
    const p = pokemonMap.get(pId);
    if (p && !seenSpecies.has(p.speciesId)) {
      seenSpecies.add(p.speciesId);
      orderedSpeciesIds.push(p.speciesId);
    }
  }

  // Just in case any species were missed due to disconnected variants, append them
  for (const sId of speciesIds) {
    if (!seenSpecies.has(sId)) {
      orderedSpeciesIds.push(sId);
    }
  }

  return orderedSpeciesIds;
}
