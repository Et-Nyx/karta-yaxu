import { drizzle } from 'drizzle-orm/better-sqlite3';
import Database from 'better-sqlite3';
import * as schema from './schema/index';

const client = new Database(process.env.DATABASE_URL || './data/rom.db');
client.pragma('journal_mode = WAL');

export const db = drizzle(client, { schema });
