import { sqliteTable, text, integer, primaryKey } from 'drizzle-orm/sqlite-core';
import { languages, generations, versionGroups } from './game-metadata';

// ── Items ──────────────────────────────────────────────────────────────────

export const items = sqliteTable('items', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  cost: integer('cost'),
  flingPower: integer('fling_power'),
  category: text('category'),
  spriteUrl: text('sprite_url'),
});

export const itemNames = sqliteTable('item_names', {
  itemId: integer('item_id').notNull().references(() => items.id),
  languageId: integer('language_id').notNull().references(() => languages.id),
  name: text('name').notNull(),
}, (t) => ({
  pk: primaryKey({ columns: [t.itemId, t.languageId] }),
}));

export const itemFlavorTexts = sqliteTable('item_flavor_texts', {
  itemId: integer('item_id').notNull().references(() => items.id),
  flavorText: text('flavor_text').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
  versionGroupId: integer('version_group_id').notNull().references(() => versionGroups.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.itemId, t.languageId, t.versionGroupId] }),
}));

export const itemEffects = sqliteTable('item_effects', {
  itemId: integer('item_id').notNull().references(() => items.id),
  effect: text('effect').notNull(),
  shortEffect: text('short_effect').notNull(),
  languageId: integer('language_id').notNull().references(() => languages.id),
}, (t) => ({
  pk: primaryKey({ columns: [t.itemId, t.languageId] }),
}));

// ── Natures ────────────────────────────────────────────────────────────────

export const natures = sqliteTable('natures', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  pokeapiId: integer('pokeapi_id'),
  source: text('source', { enum: ['pokeapi', 'edited', 'custom'] }).notNull().default('pokeapi'),
  name: text('name').notNull().unique(),
  slug: text('slug').notNull().unique(),
  increasedStat: text('increased_stat'),
  decreasedStat: text('decreased_stat'),
  likesFlavor: text('likes_flavor'),
  hatesFlavor: text('hates_flavor'),
});
