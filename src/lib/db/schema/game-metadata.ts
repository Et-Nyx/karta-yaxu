import { sqliteTable, text, integer, primaryKey } from 'drizzle-orm/sqlite-core';

// ── Languages ──────────────────────────────────────────────────────────────

export const languages = sqliteTable('languages', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  iso639: text('iso639'),
  iso3166: text('iso3166'),
  official: integer('is_official', { mode: 'boolean' }).notNull().default(true),
});

// ── Regions ────────────────────────────────────────────────────────────────

export const regions = sqliteTable('regions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  mainGenerationId: integer('main_generation_id'),
});

export const regionNames = sqliteTable('region_names', {
  regionId: integer('region_id').notNull().references(() => regions.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.regionId, t.languageId] }),
}));

// ── Generations ────────────────────────────────────────────────────────────

export const generations = sqliteTable('generations', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  regionId: integer('region_id').references(() => regions.id),
  order: integer('order'),
});

export const generationNames = sqliteTable('generation_names', {
  generationId: integer('generation_id').notNull().references(() => generations.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.generationId, t.languageId] }),
}));

// ── Version Groups & Versions ──────────────────────────────────────────────

export const versionGroups = sqliteTable('version_groups', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  generationId: integer('generation_id').notNull().references(() => generations.id),
  order: integer('order'),
});

export const versionGroupNames = sqliteTable('version_group_names', {
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.versionGroupId, t.languageId] }),
}));

export const versions = sqliteTable('versions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
});

export const versionNames = sqliteTable('version_names', {
  versionId: integer('version_id').notNull().references(() => versions.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.versionId, t.languageId] }),
}));

// ── Pokedexes ──────────────────────────────────────────────────────────────

export const pokedexes = sqliteTable('pokedexes', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  regionId: integer('region_id').references(() => regions.id),
  isMainSeries: integer('is_main_series', { mode: 'boolean' }).notNull().default(true),
});

export const pokedexNames = sqliteTable('pokedex_names', {
  pokedexId: integer('pokedex_id').notNull().references(() => pokedexes.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
  description: text('description'),
}, (t) => ({
  pk: primaryKey({ columns: [t.pokedexId, t.languageId] }),
}));
