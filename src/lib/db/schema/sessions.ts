import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';
import { trainers } from './trainers';

export const sessions = sqliteTable('sessions', {
  id:        text('id').primaryKey(),
  userId:    text('user_id').notNull().references(() => trainers.id, { onDelete: 'cascade' }),
  expiresAt: integer('expires_at').notNull(),
});
