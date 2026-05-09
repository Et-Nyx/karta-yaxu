import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';
import { pokemon } from './pokemon';
import { abilities } from './core-data';
import { moves } from './core-data';
import { items, natures } from './items';

// ── Trainers ───────────────────────────────────────────────────────────────

export const trainers = sqliteTable('trainers', {
  id: text('id').primaryKey(),
  username: text('username').notNull().unique(),
  passwordHash: text('password_hash').notNull(),
  role: text('role', { enum: ['viewer', 'editor'] }).notNull().default('viewer'),
  isNpc: integer('is_npc', { mode: 'boolean' }).notNull().default(false),
  createdAt: integer('created_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

// ── Teams ──────────────────────────────────────────────────────────────────

export const teams = sqliteTable('teams', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  trainerId: text('trainer_id').notNull().references(() => trainers.id),
  name: text('name').notNull(),
  createdAt: integer('created_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

export const teamMembers = sqliteTable('team_members', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  teamId: integer('team_id').notNull().references(() => teams.id),
  slot: integer('slot').notNull(),
  pokemonId: integer('pokemon_id').notNull().references(() => pokemon.id),
  abilityId: integer('ability_id').notNull().references(() => abilities.id),
  natureId: integer('nature_id').notNull().references(() => natures.id),
  heldItemId: integer('held_item_id').references(() => items.id),
  move1Id: integer('move1_id').references(() => moves.id),
  move2Id: integer('move2_id').references(() => moves.id),
  move3Id: integer('move3_id').references(() => moves.id),
  move4Id: integer('move4_id').references(() => moves.id),
});

// ── Battles ────────────────────────────────────────────────────────────────

export const battles = sqliteTable('battles', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  trainerId1: text('trainer_id_1').notNull().references(() => trainers.id),
  trainerId2: text('trainer_id_2').notNull().references(() => trainers.id),
  teamId1: integer('team_id_1').notNull().references(() => teams.id),
  teamId2: integer('team_id_2').notNull().references(() => teams.id),
  winnerId: text('winner_id').references(() => trainers.id),
  log: text('log').notNull().default(''),
  playedAt: integer('played_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

// ── Audit: Edits ───────────────────────────────────────────────────────────

export const edits = sqliteTable('edits', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  trainerId: text('trainer_id').notNull().references(() => trainers.id),
  tableName: text('table_name').notNull(),
  recordId: integer('record_id').notNull(),
  previousData: text('previous_data'),
  newData: text('new_data'),
  editedAt: integer('edited_at', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});

// ── Audit: Import Log ──────────────────────────────────────────────────────

export const importLog = sqliteTable('import_log', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  entityType: text('entity_type').notNull(),
  pokeapiId: integer('pokeapi_id').notNull(),
  action: text('action', { enum: ['inserted', 'updated', 'skipped'] }).notNull(),
  reason: text('reason'),
  timestamp: integer('timestamp', { mode: 'timestamp' }).$defaultFn(() => new Date()),
});
