import { error } from '@sveltejs/kit';
import { eq, desc } from 'drizzle-orm';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
  const { username } = params;

  const [trainer] = await db.select({ id: schema.trainers.id, username: schema.trainers.username })
    .from(schema.trainers)
    .where(eq(schema.trainers.username, username))
    .limit(1);

  if (!trainer) throw error(404, 'Trainer not found');

  const recentEdits = await db.select({
    id:        schema.edits.id,
    tableName: schema.edits.tableName,
    recordId:  schema.edits.recordId,
    editedAt:  schema.edits.editedAt,
  })
  .from(schema.edits)
  .where(eq(schema.edits.trainerId, trainer.id))
  .orderBy(desc(schema.edits.editedAt))
  .limit(50);

  return { trainer: { username: trainer.username }, edits: recentEdits };
};
