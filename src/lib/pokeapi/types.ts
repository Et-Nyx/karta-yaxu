// ============================================================================
// PokeAPI Response Types
// Ref: https://pokeapi.co/docs/v2
// ============================================================================

// ── Generic Types ──────────────────────────────────────────────────────────

export interface NamedAPIResource {
  name: string;
  url: string;
}

export interface NamedAPIResourceList {
  count: number;
  next: string | null;
  previous: string | null;
  results: NamedAPIResource[];
}

export interface APIResource {
  url: string;
}

export interface Name {
  name: string;
  language: NamedAPIResource;
}

export interface FlavorText {
  flavor_text: string;
  language: NamedAPIResource;
  version_group: NamedAPIResource;
}

export interface VersionFlavorText {
  flavor_text: string;
  language: NamedAPIResource;
  version: NamedAPIResource;
}

export interface Effect {
  effect: string;
  language: NamedAPIResource;
}

export interface VerboseEffect {
  effect: string;
  short_effect: string;
  language: NamedAPIResource;
}

export interface Description {
  description: string;
  language: NamedAPIResource;
}

export interface GenerationGameIndex {
  game_index: number;
  generation: NamedAPIResource;
}

// ── Game Metadata ──────────────────────────────────────────────────────────

export interface PokeApiLanguage {
  id: number;
  name: string;
  official: boolean;
  iso639: string;
  iso3166: string;
  names: Name[];
}

export interface PokeApiGeneration {
  id: number;
  name: string;
  names: Name[];
  main_region: NamedAPIResource;
  abilities: NamedAPIResource[];
  moves: NamedAPIResource[];
  pokemon_species: NamedAPIResource[];
  types: NamedAPIResource[];
  version_groups: NamedAPIResource[];
}

export interface PokeApiRegion {
  id: number;
  name: string;
  names: Name[];
  locations: NamedAPIResource[];
  main_generation: NamedAPIResource | null;
  pokedexes: NamedAPIResource[];
  version_groups: NamedAPIResource[];
}

export interface PokeApiVersionGroup {
  id: number;
  name: string;
  order: number;
  generation: NamedAPIResource;
  move_learn_methods: NamedAPIResource[];
  pokedexes: NamedAPIResource[];
  regions: NamedAPIResource[];
  versions: NamedAPIResource[];
}

export interface PokeApiVersion {
  id: number;
  name: string;
  names: Name[];
  version_group: NamedAPIResource;
}

export interface PokedexEntry {
  entry_number: number;
  pokemon_species: NamedAPIResource;
}

export interface PokeApiPokedex {
  id: number;
  name: string;
  is_main_series: boolean;
  descriptions: Description[];
  names: Name[];
  pokemon_entries: PokedexEntry[];
  region: NamedAPIResource | null;
  version_groups: NamedAPIResource[];
}

// ── Core Data ──────────────────────────────────────────────────────────────

export interface TypeRelations {
  no_damage_to: NamedAPIResource[];
  half_damage_to: NamedAPIResource[];
  double_damage_to: NamedAPIResource[];
  no_damage_from: NamedAPIResource[];
  half_damage_from: NamedAPIResource[];
  double_damage_from: NamedAPIResource[];
}

export interface PokeApiType {
  id: number;
  name: string;
  names: Name[];
  damage_relations: TypeRelations;
  game_indices: GenerationGameIndex[];
  generation: NamedAPIResource;
  move_damage_class: NamedAPIResource | null;
  moves: NamedAPIResource[];
  pokemon: Array<{ pokemon: NamedAPIResource; slot: number }>;
}

export interface PokeApiAbility {
  id: number;
  name: string;
  names: Name[];
  is_main_series: boolean;
  generation: NamedAPIResource;
  effect_entries: VerboseEffect[];
  flavor_text_entries: FlavorText[];
  pokemon: Array<{ is_hidden: boolean; pokemon: NamedAPIResource; slot: number }>;
}

export interface MoveStatChange {
  change: number;
  stat: NamedAPIResource;
}

export interface MoveMetaData {
  ailment: NamedAPIResource;
  category: NamedAPIResource;
  min_hits: number | null;
  max_hits: number | null;
  min_turns: number | null;
  max_turns: number | null;
  drain: number;
  healing: number;
  crit_rate: number;
  ailment_chance: number;
  flinch_chance: number;
  stat_chance: number;
}

export interface MoveFlavorText {
  flavor_text: string;
  language: NamedAPIResource;
  version_group: NamedAPIResource;
}

export interface PokeApiMove {
  id: number;
  name: string;
  names: Name[];
  accuracy: number | null;
  effect_chance: number | null;
  pp: number;
  priority: number;
  power: number | null;
  damage_class: NamedAPIResource;
  effect_entries: VerboseEffect[];
  flavor_text_entries: MoveFlavorText[];
  generation: NamedAPIResource;
  meta: MoveMetaData | null;
  stat_changes: MoveStatChange[];
  target: NamedAPIResource;
  type: NamedAPIResource;
}

export interface ItemSprites {
  default: string | null;
}

export interface PokeApiItem {
  id: number;
  name: string;
  names: Name[];
  cost: number;
  fling_power: number | null;
  fling_effect: NamedAPIResource | null;
  category: NamedAPIResource;
  effect_entries: VerboseEffect[];
  flavor_text_entries: VersionFlavorText[];
  sprites: ItemSprites;
}

export interface NatureStatAffect {
  name: string;
  url: string;
}

export interface PokeApiNature {
  id: number;
  name: string;
  names: Name[];
  decreased_stat: NamedAPIResource | null;
  increased_stat: NamedAPIResource | null;
  likes_flavor: NamedAPIResource | null;
  hates_flavor: NamedAPIResource | null;
}

export interface PokeApiEggGroup {
  id: number;
  name: string;
  names: Name[];
  pokemon_species: NamedAPIResource[];
}

export interface PokeApiMoveLearnMethod {
  id: number;
  name: string;
  names: Name[];
  descriptions: Description[];
  version_groups: NamedAPIResource[];
}

export interface PokeApiEncounterMethod {
  id: number;
  name: string;
  names: Name[];
  order: number;
}

export interface PokeApiEncounterCondition {
  id: number;
  name: string;
  names: Name[];
  values: NamedAPIResource[];
}

export interface PokeApiEncounterConditionValue {
  id: number;
  name: string;
  names: Name[];
  condition: NamedAPIResource;
}

// ── Pokémon ────────────────────────────────────────────────────────────────

export interface PokemonStat {
  base_stat: number;
  effort: number;
  stat: NamedAPIResource;
}

export interface PokemonType {
  slot: number;
  type: NamedAPIResource;
}

export interface PokemonAbility {
  ability: NamedAPIResource;
  is_hidden: boolean;
  slot: number;
}

export interface PokemonMoveVersion {
  level_learned_at: number;
  move_learn_method: NamedAPIResource;
  version_group: NamedAPIResource;
}

export interface PokemonMove {
  move: NamedAPIResource;
  version_group_details: PokemonMoveVersion[];
}

export interface PokemonSprites {
  front_default: string | null;
  front_shiny: string | null;
  back_default?: string | null;
  back_shiny?: string | null;
  other?: {
    'official-artwork'?: {
      front_default: string | null;
      front_shiny: string | null;
    };
  };
}

export interface PokeApiPokemon {
  id: number;
  name: string;
  height: number;
  weight: number;
  is_default: boolean;
  order: number;
  stats: PokemonStat[];
  types: PokemonType[];
  abilities: PokemonAbility[];
  moves: PokemonMove[];
  forms: NamedAPIResource[];
  species: NamedAPIResource;
  sprites: PokemonSprites;
}

export interface Genus {
  genus: string;
  language: NamedAPIResource;
}

export interface PalParkEncounter {
  area: NamedAPIResource;
  base_score: number;
  rate: number;
}

export interface PokeApiPokemonSpecies {
  id: number;
  name: string;
  order: number;
  gender_rate: number;
  capture_rate: number;
  base_happiness: number;
  is_baby: boolean;
  is_legendary: boolean;
  is_mythical: boolean;
  hatch_counter: number;
  has_gender_differences: boolean;
  forms_switchable: boolean;
  growth_rate: NamedAPIResource;
  pokedex_numbers: Array<{ entry_number: number; pokedex: NamedAPIResource }>;
  egg_groups: NamedAPIResource[];
  color: NamedAPIResource;
  shape: NamedAPIResource | null;
  habitat: NamedAPIResource | null;
  generation: NamedAPIResource;
  names: Name[];
  flavor_text_entries: VersionFlavorText[];
  genera: Genus[];
  evolves_from_species: NamedAPIResource | null;
  evolution_chain: APIResource;
  varieties: Array<{ is_default: boolean; pokemon: NamedAPIResource }>;
}

export interface PokemonFormSprites {
  front_default: string | null;
  front_shiny: string | null;
  back_default?: string | null;
  back_shiny?: string | null;
}

export interface PokeApiPokemonForm {
  id: number;
  name: string;
  form_name: string;
  is_default: boolean;
  is_battle_only: boolean;
  is_mega: boolean;
  order: number;
  pokemon: NamedAPIResource;
  sprites: PokemonFormSprites;
  types: PokemonType[];
  version_group: NamedAPIResource;
}

// ── Evolution ──────────────────────────────────────────────────────────────

export interface EvolutionDetail {
  gender: number | null;
  held_item: NamedAPIResource | null;
  item: NamedAPIResource | null;
  known_move: NamedAPIResource | null;
  known_move_type: NamedAPIResource | null;
  location: NamedAPIResource | null;
  min_affection: number | null;
  min_beauty: number | null;
  min_happiness: number | null;
  min_level: number | null;
  needs_overworld_rain: boolean;
  party_species: NamedAPIResource | null;
  party_type: NamedAPIResource | null;
  relative_physical_stats: number | null;
  time_of_day: string;
  trade_species: NamedAPIResource | null;
  trigger: NamedAPIResource;
  turn_upside_down: boolean;
}

export interface ChainLink {
  is_baby: boolean;
  species: NamedAPIResource;
  evolution_details: EvolutionDetail[];
  evolves_to: ChainLink[];
}

export interface PokeApiEvolutionChain {
  id: number;
  baby_trigger_item: NamedAPIResource | null;
  chain: ChainLink;
}

// ── Locations ──────────────────────────────────────────────────────────────

export interface PokeApiLocation {
  id: number;
  name: string;
  names: Name[];
  region: NamedAPIResource | null;
  areas: NamedAPIResource[];
  game_indices: GenerationGameIndex[];
}

export interface EncounterVersionDetail {
  rate: number;
  version: NamedAPIResource;
}

export interface EncounterSlot {
  pokemon: NamedAPIResource;
  version_details: Array<{
    encounter_details: Array<{
      chance: number;
      condition_values: NamedAPIResource[];
      max_level: number;
      min_level: number;
      method: NamedAPIResource;
    }>;
    max_chance: number;
    version: NamedAPIResource;
  }>;
}

export interface PokeApiLocationArea {
  id: number;
  name: string;
  names: Name[];
  game_index: number;
  location: NamedAPIResource;
  encounter_method_rates: Array<{
    encounter_method: NamedAPIResource;
    version_details: EncounterVersionDetail[];
  }>;
  pokemon_encounters: EncounterSlot[];
}
