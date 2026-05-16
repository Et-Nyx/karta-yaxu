import { error } from '@sveltejs/kit';
import { eq, count, or } from 'drizzle-orm';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
  const { username } = params;

  const [trainer] = await db.select({
    id:        schema.trainers.id,
    username:  schema.trainers.username,
    role:      schema.trainers.role,
    createdAt: schema.trainers.createdAt,
  })
  .from(schema.trainers)
  .where(eq(schema.trainers.username, username))
  .limit(1);

  if (!trainer) throw error(404, 'Trainer not found');

  const [{ teamCount }] = await db
    .select({ teamCount: count() })
    .from(schema.teams)
    .where(eq(schema.teams.trainerId, trainer.id));

  const [{ battleCount }] = await db
    .select({ battleCount: count() })
    .from(schema.battles)
    .where(or(
      eq(schema.battles.trainerId1, trainer.id),
      eq(schema.battles.trainerId2, trainer.id),
    ));

  const [{ editCount }] = await db
    .select({ editCount: count() })
    .from(schema.edits)
    .where(eq(schema.edits.trainerId, trainer.id));

  return {
    trainer: {
      username:  trainer.username,
      role:      trainer.role,
      createdAt: trainer.createdAt,
    },
    stats: { teamCount, battleCount, editCount },
    isOwnProfile: locals.user?.username === trainer.username,
  };
};
