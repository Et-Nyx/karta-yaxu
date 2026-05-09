import { sqliteTable, text, integer, primaryKey } from 'drizzle-orm/sqlite-core';
import { languages, generations, versionGroups, pokedexes, versions } from './game-metadata';
import { types, abilities, moves, moveLearnMethods } from './core-data';
import { items } from './items';
import { locationAreas, encounterMethods, encounterConditionValues } from './locations';

// ── Egg Groups ─────────────────────────────────────────────────────────────

export const eggGroups = sqliteTable('egg_groups', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
});

// ── Evolution Chains ───────────────────────────────────────────────────────

export const evolutionChains = sqliteTable('evolution_chains', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  babyTriggerItemId: integer('baby_trigger_item_id').references(() => items.id),
});

// ── Pokemon Species ────────────────────────────────────────────────────────

export const pokemonSpecies = sqliteTable('pokemon_species', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  genus: text('genus'),
  dexNumber: integer('dex_number').notNull(),
  evolutionChainId: integer('evolution_chain_id').references(() => evolutionChains.id),
  evolvesFromSpeciesId: integer('evolves_from_species_id'),
  isBaby: integer('is_baby', { mode: 'boolean' }).notNull().default(false),
  isLegendary: integer('is_legendary', { mode: 'boolean' }).notNull().default(false),
  isMythical: integer('is_mythical', { mode: 'boolean' }).notNull().default(false),
  habitat: text('habitat'),
  color: text('color'),
  shape: text('shape'),
  growthRate: text('growth_rate'),
  baseHappiness: integer('base_happiness'),
  captureRate: integer('capture_rate'),
  genderRate: integer('gender_rate'),
  hatchCounter: integer('hatch_counter'),
  generationId: integer('generation_id').references(() => generations.id),
  order: integer('order'),
});

export const pokemonFlavorTexts = sqliteTable('pokemon_flavor_texts', {
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.speciesId, t.languageId, t.versionGroupId] }),
}));

export const pokemonGenera = sqliteTable('pokemon_genera', {
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  genus: text('genus').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.speciesId, t.languageId] }),
}));

export const speciesEggGroups = sqliteTable('species_egg_groups', {
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  eggGroupId: integer('egg_group_id').notNull().references(() => eggGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.speciesId, t.eggGroupId] }),
}));

// ── Pokemon ────────────────────────────────────────────────────────────────

export const pokemon = sqliteTable('pokemon', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  height: integer('height').notNull(),
  weight: integer('weight').notNull(),
  hp: integer('hp').notNull(),
  atk: integer('atk').notNull(),
  def: integer('def').notNull(),
  spa: integer('spa').notNull(),
  spd: integer('spd').notNull(),
  spe: integer('spe').notNull(),
  spriteDefault: text('sprite_default'),
  spriteShiny: text('sprite_shiny'),
  spriteArtwork: text('sprite_artwork'),
  isDefault: integer('is_default', { mode: 'boolean' }).notNull().default(true),
  order: integer('order'),
});

export const pokemonTypes = sqliteTable('pokemon_types', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  typeId: integer('type_id').notNull().references(() => types.id),
  slot: integer('slot').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.slot] }),
}));

export const pokemonAbilities = sqliteTable('pokemon_abilities', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  slot: integer('slot').notNull(),
  isHidden: integer('is_hidden', { mode: 'boolean' }).notNull().default(false),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.abilityId] }),
}));

export const pokemonStats = sqliteTable('pokemon_stats', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  statName: text('stat_name').notNull(),
  baseStat: integer('base_stat').notNull(),
  effort: integer('effort').notNull().default(0),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.statName] }),
}));

export const pokemonMoves = sqliteTable('pokemon_moves', {
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  moveId: integer('move_id').notNull().references(() => moves.id),
  learnMethodId: integer('learn_method_id').notNull().references(() => moveLearnMethods.id),
  level: integer('level'),
  versionGroupId: integer('version_group_id').references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokemonId, t.moveId, t.learnMethodId, t.level] }),
}));

// ── Pokemon Forms ──────────────────────────────────────────────────────────

export const pokemonForms = sqliteTable('pokemon_forms', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  formName: text('form_name'),
  isDefault: integer('is_default', { mode: 'boolean' }).notNull().default(false),
  isBattleOnly: integer('is_battle_only', { mode: 'boolean' }).notNull().default(false),
  isMega: integer('is_mega', { mode: 'boolean' }).notNull().default(false),
  order: integer('order'),
  spriteDefault: text('sprite_default'),
  spriteShiny: text('sprite_shiny'),
  spriteArtwork: text('sprite_artwork'),
});

// ── Evolution Details ──────────────────────────────────────────────────────

export const evolutionDetails = sqliteTable('evolution_details', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  chainId: integer('chain_id').notNull().references(() => evolutionChains.id),
  fromSpeciesId: integer('from_species_id').notNull().references(() => pokemonSpecies.id),
  toSpeciesId: integer('to_species_id').notNull().references(() => pokemonSpecies.id),
  trigger: text('trigger').notNull(),
  minLevel: integer('min_level'),
  minHappiness: integer('min_happiness'),
  minBeauty: integer('min_beauty'),
  minAffection: integer('min_affection'),
  itemId: integer('item_id').references(() => items.id),
  heldItemId: integer('held_item_id').references(() => items.id),
  knownMoveId: integer('known_move_id').references(() => moves.id),
  knownMoveTypeId: integer('known_move_type_id').references(() => types.id),
  location: text('location'),
  timeOfDay: text('time_of_day'),
  gender: integer('gender'),
  relativePhysicalStats: integer('relative_physical_stats'),
  needsOverworldRain: integer('needs_overworld_rain', { mode: 'boolean' }).default(false),
  needsMultiplayer: integer('needs_multiplayer', { mode: 'boolean' }).default(false),
  turnUpsideDown: integer('turn_upside_down', { mode: 'boolean' }).default(false),
  tradeSpeciesId: integer('trade_species_id').references(() => pokemonSpecies.id),
  partySpeciesId: integer('party_species_id').references(() => pokemonSpecies.id),
  partyTypeId: integer('party_type_id').references(() => types.id),
  region: text('region'),
  baseFormId: integer('base_form_id').references(() => pokemonForms.id),
  usedMoveId: integer('used_move_id').references(() => moves.id),
  minMoveCount: integer('min_move_count'),
  minSteps: integer('min_steps'),
  minDamageTaken: integer('min_damage_taken'),
});

// ── Pokedex Entries ────────────────────────────────────────────────────────

export const pokedexEntries = sqliteTable('pokedex_entries', {
  pokedexId: integer('pokedex_id').notNull().references(() => pokedexes.id),
  speciesId: integer('species_id').notNull().references(() => pokemonSpecies.id),
  entryNumber: integer('entry_number').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokedexId, t.speciesId] }),
}));

// ── Encounters ─────────────────────────────────────────────────────────────

export const encounters = sqliteTable('encounters', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  locationAreaId: integer('location_area_id').notNull().references(() => locationAreas.id),
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  versionId: integer('version_id').notNull().references(() => versions.id),
  encounterMethodId: integer('encounter_method_id').notNull().references(() => encounterMethods.id),
  conditionValueId: integer('condition_value_id').references(() => encounterConditionValues.id),
  minLevel: integer('min_level').notNull(),
  maxLevel: integer('max_level').notNull(),
  chance: integer('chance').notNull(),
  slot: integer('slot'),
});
