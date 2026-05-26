import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import { and, eq, sql } from 'drizzle-orm';
import { requireEditor } from '$lib/auth/guards';
import * as client from '$lib/pokeapi/client';
import { importPokemonById } from '$lib/pokeapi/import-pokemon';
import { auditStore } from '$lib/server/context';
import { importEvents } from '$lib/pokeapi/import-events';

export const POST: RequestHandler = async ({ request, locals }) => {
  requireEditor(locals);

  try {
    const body = await request.json().catch(() => ({}));
    const comment = String(body.comment || '').trim();

    if (!comment) {
      return json({ success: false, error: 'A commit comment is required.' }, { status: 400 });
    }

    // 1. Fetch all fetched staged imports for this trainer
    const staged = await db
      .select()
      .from(schema.stagedImports)
      .where(
        and(
          eq(schema.stagedImports.trainerId, locals.user!.id),
          eq(schema.stagedImports.status, 'fetched')
        )
      );

    if (staged.length === 0) {
      return json({ success: false, error: 'No fetched staged imports found.' }, { status: 400 });
    }

    // 2. Populate client-side cache so ensure functions make 0 network requests
    client.clearCache();
    for (const s of staged) {
      if (!s.payload) continue;
      const parsed = JSON.parse(s.payload);
      if (s.entityType === 'pokemon') {
        client.populateCache(`/pokemon-species/${s.pokeapiId}`, parsed.species);
        client.populateCache(`/pokemon/${s.pokeapiId}`, parsed.pokemon);
      } else {
        client.populateCache(`/${s.entityType}/${s.pokeapiId}`, parsed);
      }
    }

    // 3. Wrap everything in a single SQLite transaction & propagation context
    let importedCount = 0;
    await auditStore.run({ comment }, async () => {
      // Execute native SQLite Transaction
      await db.run(sql`BEGIN TRANSACTION`);
      try {
        let index = 0;
        for (const s of staged) {
          if (s.entityType === 'pokemon') {
            let name = 'Pokémon';
            try {
              if (s.payload) {
                const parsed = JSON.parse(s.payload);
                name = parsed.species?.name || 'Pokémon';
              }
            } catch {}

            // Capitalize first letter of Pokémon name
            const capitalizedName = name.charAt(0).toUpperCase() + name.slice(1);

            importEvents.emit('log', {
              entityType: 'migrating',
              pokeapiId: s.pokeapiId,
              action: 'inserted',
              comment: JSON.stringify({
                name: capitalizedName,
                id: s.pokeapiId,
                index: index + 1,
                total: staged.length
              }),
              timestamp: new Date().toISOString()
            });

            await importPokemonById(s.pokeapiId, { forceRefresh: true });
            importedCount++;
            index++;
          }
        }

        // Clean up staged imports on successful application
        await db
          .delete(schema.stagedImports)
          .where(eq(schema.stagedImports.trainerId, locals.user!.id));

        await db.run(sql`COMMIT`);
      } catch (err: any) {
        await db.run(sql`ROLLBACK`);
        throw err;
      }
    });

    return json({
      success: true,
      message: `Successfully migrated ${importedCount} Pokémon to the database atomically!`,
    });
  } catch (e: any) {
    console.error('Commit import error:', e);
    return json(
      { success: false, error: e.message || 'An error occurred during migration.' },
      { status: 500 }
    );
  }
};
