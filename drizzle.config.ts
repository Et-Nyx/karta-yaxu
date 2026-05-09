import { defineConfig } from 'drizzle-kit';

export default defineConfig({
	schema: './src/lib/db/schema',
	out: './drizzle',
	dialect: 'sqlite',
	dbCredentials: {
		url: process.env.DATABASE_URL || './data/rom.db'
	}
});
