import { error, fail, redirect } from '@sveltejs/kit';
import { eq, and } from 'drizzle-orm';
import { db } from '$lib/db';
import { trainers } from '$lib/db/schema/trainers';
import { requireEditor } from '$lib/auth/guards';
import type { PageServerLoad, Actions } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
  requireEditor(locals);

  const [trainer] = await db
    .select({ id: trainers.id, username: trainers.username, createdAt: trainers.createdAt })
    .from(trainers)
    .where(and(eq(trainers.username, params.username), eq(trainers.isNpc, true)))
    .limit(1);

  if (!trainer) error(404, 'NPC trainer not found');

  return { trainer };
};

export const actions: Actions = {
  update: async ({ request, params, locals }) => {
    requireEditor(locals);
    const username = ((await request.formData()).get('username') as string)?.trim();

    if (!username || username.length < 1 || username.length > 30) {
      return fail(400, { updateError: 'Name must be 1–30 characters', values: { username } });
    }

    const [current] = await db.select({ id: trainers.id })
      .from(trainers)
      .where(and(eq(trainers.username, params.username), eq(trainers.isNpc, true)))
      .limit(1);
    if (!current) error(404, 'NPC trainer not found');

    if (username !== params.username) {
      const [clash] = await db.select({ id: trainers.id })
        .from(trainers).where(eq(trainers.username, username)).limit(1);
      if (clash) return fail(400, { updateError: 'Username already in use', values: { username } });
    }

    await db.update(trainers).set({ username }).where(eq(trainers.id, current.id));

    if (username !== params.username) redirect(303, `/editor/trainers/${username}`);
    return { updateSuccess: true };
  },
};
