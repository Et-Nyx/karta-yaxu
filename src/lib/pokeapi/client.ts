import type {
  NamedAPIResourceList,
  PokeApiLanguage, PokeApiGeneration, PokeApiRegion,
  PokeApiVersionGroup, PokeApiVersion, PokeApiPokedex,
  PokeApiType, PokeApiAbility, PokeApiMove, PokeApiItem, PokeApiNature,
  PokeApiPokemon, PokeApiPokemonSpecies, PokeApiPokemonForm,
  PokeApiEvolutionChain,
  PokeApiLocation, PokeApiLocationArea,
  PokeApiEncounterMethod, PokeApiEncounterCondition, PokeApiEncounterConditionValue,
  PokeApiMoveLearnMethod, PokeApiEggGroup,
} from './types';

// ============================================================================
// PokeAPI HTTP Client
// - In-memory cache per session
// - Rate limiting (80 req/min)
// - Retry with exponential backoff
// ============================================================================

const BASE_URL = 'https://pokeapi.co/api/v2';

// ── Cache ──────────────────────────────────────────────────────────────────

const cache = new Map<string, unknown>();

export function clearCache(): void {
  cache.clear();
}

// ── Rate Limiter ───────────────────────────────────────────────────────────

const RATE_LIMIT = 80; // requests per minute
const RATE_WINDOW = 60_000; // 1 minute in ms
const timestamps: number[] = [];

async function waitForRateLimit(): Promise<void> {
  const now = Date.now();
  // Remove timestamps older than the window
  while (timestamps.length > 0 && timestamps[0]! < now - RATE_WINDOW) {
    timestamps.shift();
  }
  if (timestamps.length >= RATE_LIMIT) {
    const oldest = timestamps[0]!;
    const waitMs = oldest + RATE_WINDOW - now + 100; // +100ms buffer
    await sleep(waitMs);
  }
  timestamps.push(Date.now());
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ── Core Fetch ─────────────────────────────────────────────────────────────

const MAX_RETRIES = 3;
const INITIAL_BACKOFF = 1000;

async function fetchFromApi<T>(path: string): Promise<T> {
  // Check cache
  if (cache.has(path)) {
    return cache.get(path) as T;
  }

  await waitForRateLimit();

  let lastError: Error | null = null;

  for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
    try {
      const response = await fetch(`${BASE_URL}${path}`);

      if (response.status === 404) {
        throw new Error(`PokeAPI 404: ${path} not found`);
      }

      if (!response.ok) {
        throw new Error(`PokeAPI ${response.status}: ${response.statusText} for ${path}`);
      }

      const data = (await response.json()) as T;
      cache.set(path, data);
      return data;
    } catch (error) {
      lastError = error as Error;
      if ((error as Error).message.includes('404')) {
        throw error; // Don't retry 404s
      }
      const backoffMs = INITIAL_BACKOFF * Math.pow(2, attempt);
      console.warn(`PokeAPI fetch failed (attempt ${attempt + 1}/${MAX_RETRIES}): ${path} — retrying in ${backoffMs}ms`);
      await sleep(backoffMs);
    }
  }

  throw lastError ?? new Error(`Failed to fetch ${path} after ${MAX_RETRIES} attempts`);
}

// ── URL Helpers ────────────────────────────────────────────────────────────

/**
 * Extracts the numeric ID from a PokeAPI URL.
 * e.g. "https://pokeapi.co/api/v2/region/3/" → 3
 */
export function extractIdFromUrl(url: string): number {
  const match = url.match(/\/(\d+)\/?$/);
  if (!match) throw new Error(`Cannot extract ID from URL: ${url}`);
  return parseInt(match[1]!, 10);
}

// ── Typed Fetch Functions ──────────────────────────────────────────────────

// Game Metadata
export const fetchLanguage = (id: number) => fetchFromApi<PokeApiLanguage>(`/language/${id}`);
export const fetchGeneration = (id: number) => fetchFromApi<PokeApiGeneration>(`/generation/${id}`);
export const fetchRegion = (id: number) => fetchFromApi<PokeApiRegion>(`/region/${id}`);
export const fetchVersionGroup = (id: number) => fetchFromApi<PokeApiVersionGroup>(`/version-group/${id}`);
export const fetchVersion = (id: number) => fetchFromApi<PokeApiVersion>(`/version/${id}`);
export const fetchPokedex = (id: number) => fetchFromApi<PokeApiPokedex>(`/pokedex/${id}`);

// Core Data
export const fetchType = (id: number) => fetchFromApi<PokeApiType>(`/type/${id}`);
export const fetchAbility = (id: number) => fetchFromApi<PokeApiAbility>(`/ability/${id}`);
export const fetchMove = (id: number) => fetchFromApi<PokeApiMove>(`/move/${id}`);
export const fetchItem = (id: number) => fetchFromApi<PokeApiItem>(`/item/${id}`);
export const fetchNature = (id: number) => fetchFromApi<PokeApiNature>(`/nature/${id}`);
export const fetchEncounterMethod = (id: number) => fetchFromApi<PokeApiEncounterMethod>(`/encounter-method/${id}`);
export const fetchEncounterCondition = (id: number) => fetchFromApi<PokeApiEncounterCondition>(`/encounter-condition/${id}`);
export const fetchEncounterConditionValue = (id: number) => fetchFromApi<PokeApiEncounterConditionValue>(`/encounter-condition-value/${id}`);
export const fetchMoveLearnMethod = (id: number) => fetchFromApi<PokeApiMoveLearnMethod>(`/move-learn-method/${id}`);
export const fetchEggGroup = (id: number) => fetchFromApi<PokeApiEggGroup>(`/egg-group/${id}`);

// Pokémon
export const fetchPokemon = (id: number) => fetchFromApi<PokeApiPokemon>(`/pokemon/${id}`);
export const fetchPokemonSpecies = (id: number) => fetchFromApi<PokeApiPokemonSpecies>(`/pokemon-species/${id}`);
export const fetchPokemonForm = (id: number) => fetchFromApi<PokeApiPokemonForm>(`/pokemon-form/${id}`);
export const fetchEvolutionChain = (id: number) => fetchFromApi<PokeApiEvolutionChain>(`/evolution-chain/${id}`);

// Locations
export const fetchLocation = (id: number) => fetchFromApi<PokeApiLocation>(`/location/${id}`);
export const fetchLocationArea = (id: number) => fetchFromApi<PokeApiLocationArea>(`/location-area/${id}`);

// Listing (for batch imports)
export async function fetchList(endpoint: string, limit: number = 10000): Promise<NamedAPIResourceList> {
  return fetchFromApi<NamedAPIResourceList>(`/${endpoint}?limit=${limit}`);
}
