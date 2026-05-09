import { sqliteTable, text, integer, primaryKey } from 'drizzle-orm/sqlite-core';
import { languages } from './game-metadata';
import { regions } from './game-metadata';

// ── Locations ──────────────────────────────────────────────────────────────

export const locations = sqliteTable('locations', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  regionId: integer('region_id').references(() => regions.id),
});

export const locationNames = sqliteTable('location_names', {
  locationId: integer('location_id').notNull().references(() => locations.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.locationId, t.languageId] }),
}));

// ── Location Areas ─────────────────────────────────────────────────────────

export const locationAreas = sqliteTable('location_areas', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  locationId: integer('location_id').notNull().references(() => locations.id),
  gameIndex: integer('game_index'),
});

export const locationAreaNames = sqliteTable('location_area_names', {
  locationAreaId: integer('location_area_id').notNull().references(() => locationAreas.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.locationAreaId, t.languageId] }),
}));

// ── Encounter Methods & Conditions ─────────────────────────────────────────

export const encounterMethods = sqliteTable('encounter_methods', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  order: integer('order'),
});

export const encounterMethodNames = sqliteTable('encounter_method_names', {
  encounterMethodId: integer('encounter_method_id').notNull().references(() => encounterMethods.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
  description: text('description'),
}, (t) => ({
  pk: primaryKey({ columns: [t.encounterMethodId, t.languageId] }),
}));

export const encounterConditions = sqliteTable('encounter_conditions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
});

export const encounterConditionValues = sqliteTable('encounter_condition_values', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  conditionId: integer('condition_id').notNull().references(() => encounterConditions.id),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
});
