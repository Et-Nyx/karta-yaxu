import {
  ensureAbility,
  ensureItem,
  ensureMove,
  ensurePokemon,
  ensurePokemonSpecies,
  ensureType,
} from '$lib/pokeapi/import-pokemon';
import { ensureNature } from '$lib/pokeapi/import-service';

export type ResourceTable =
  | 'types'
  | 'natures'
  | 'abilities'
  | 'items'
  | 'moves'
  | 'pokemon'
  | 'pokemon_species';

export async function reimportEntity(table: ResourceTable, pokeapiId: number): Promise<number> {
  switch (table) {
    case 'types':           return await ensureType(pokeapiId, { forceRefresh: true });
    case 'natures':         return await ensureNature(pokeapiId, { forceRefresh: true });
    case 'abilities':       return await ensureAbility(pokeapiId, { forceRefresh: true });
    case 'items':           return await ensureItem(pokeapiId, { forceRefresh: true });
    case 'moves':           return await ensureMove(pokeapiId, { forceRefresh: true });
    case 'pokemon':         return await ensurePokemon(pokeapiId, { forceRefresh: true });
    case 'pokemon_species': return await ensurePokemonSpecies(pokeapiId, { forceRefresh: true });
  }
}
