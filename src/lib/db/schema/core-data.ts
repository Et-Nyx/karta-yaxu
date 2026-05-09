import { sqliteTable, text, integer, real, primaryKey } from 'drizzle-orm/sqlite-core';
import { languages, generations, versionGroups } from './game-metadata';

// ── Types ──────────────────────────────────────────────────────────────────

export const types = sqliteTable('types', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  hexColor: text('hexcolor'),
  iconUrl: text('icon_url'),
  moveDamageClass: text('move_damage_class', { enum: ['physical', 'special', 'status'] }),
  generationId: integer('generation_id').references(() => generations.id),
});

export const typeEffectiveness = sqliteTable('type_effectiveness', {
  attackerId: integer('attacker_id').notNull().references(() => types.id),
  defenderId: integer('defender_id').notNull().references(() => types.id),
  multiplier: real('multiplier').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.attackerId, t.defenderId] }),
}));

export const typeNames = sqliteTable('type_names', {
  typeId: integer('type_id').notNull().references(() => types.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.typeId, t.languageId] }),
}));

// ── Abilities ──────────────────────────────────────────────────────────────

export const abilities = sqliteTable('abilities', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  isMainSeries: integer('is_main_series', { mode: 'boolean' }).notNull().default(true),
  generationId: integer('generation_id').references(() => generations.id),
});

export const abilityNames = sqliteTable('ability_names', {
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.abilityId, t.languageId] }),
}));

export const abilityFlavorTexts = sqliteTable('ability_flavor_texts', {
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.abilityId, t.languageId, t.versionGroupId] }),
}));

export const abilityEffects = sqliteTable('ability_effects', {
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  effect: text('effect').notNull(),
  shortEffect: text('short_effect').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.abilityId, t.languageId] }),
}));

// ── Moves ──────────────────────────────────────────────────────────────────

export const moves = sqliteTable('moves', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  typeId: integer('type_id').notNull().references(() => types.id),
  damageClass: text('damage_class', { enum: ['physical', 'special', 'status'] }).notNull(),
  power: integer('power'),
  accuracy: integer('accuracy'),
  pp: integer('pp').notNull(),
  priority: integer('priority').notNull().default(0),
  target: text('target'),
  generationId: integer('generation_id').references(() => generations.id),
});

export const moveNames = sqliteTable('move_names', {
  moveId: integer('move_id').notNull().references(() => moves.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveId, t.languageId] }),
}));

export const moveFlavorTexts = sqliteTable('move_flavor_texts', {
  moveId: integer('move_id').notNull().references(() => moves.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveId, t.languageId, t.versionGroupId] }),
}));

export const moveEffects = sqliteTable('move_effects', {
  moveId: integer('move_id').notNull().references(() => moves.id),
  effect: text('effect').notNull(),
  shortEffect: text('short_effect').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveId, t.languageId] }),
}));

export const moveLearnMethods = sqliteTable('move_learn_methods', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  description: text('description'),
});

export const moveLearnMethodNames = sqliteTable('move_learn_method_names', {
  moveLearnMethodId: integer('move_learn_method_id').notNull().references(() => moveLearnMethods.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
  description: text('description'),
}, (t) => ({
  pk: primaryKey({ columns: [t.moveLearnMethodId, t.languageId] }),
}));
