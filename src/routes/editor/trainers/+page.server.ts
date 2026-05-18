import { fail, redirect } from '@sveltejs/kit';
import { asc, count, eq, and } from 'drizzle-orm';
import { randomBytes } from 'node:crypto';
import { generateId } from 'lucia';
import { db } from '$lib/db';
import { trainers } from '$lib/db/schema/trainers';
import { requireEditor } from '$lib/auth/guards';
import type { PageServerLoad, Actions } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
  requireEditor(locals);

  const npcs = await db.select({
    id:        trainers.id,
    username:  trainers.username,
    createdAt: trainers.createdAt,
  })
    .from(trainers)
    .where(eq(trainers.isNpc, true))
    .orderBy(asc(trainers.createdAt));

  const [{ total }] = await db.select({ total: count() })
    .from(trainers)
    .where(eq(trainers.isNpc, true));

  return { npcs, total };
};

export const actions: Actions = {
  create: async ({ request, locals }) => {
    requireEditor(locals);
    const username = ((await request.formData()).get('username') as string)?.trim();
    if (!username || username.length < 1 || username.length > 30) {
      return fail(400, { createError: 'Name must be 1–30 characters' });
    }

    const [existing] = await db.select({ id: trainers.id })
      .from(trainers).where(eq(trainers.username, username)).limit(1);
    if (existing) return fail(400, { createError: 'Username already in use' });

    const passwordHash = randomBytes(32).toString('hex') + '.' + randomBytes(16).toString('hex');
    await db.insert(trainers).values({
      id: generateId(15),
      username,
      passwordHash,
      role: 'viewer',
      isNpc: true,
    });

    redirect(303, `/editor/trainers/${username}`);
  },

  delete: async ({ request, locals }) => {
    requireEditor(locals);
    const id = (await request.formData()).get('id') as string;
    if (!id) return fail(400, { error: 'Invalid' });
    await db.delete(trainers).where(and(eq(trainers.id, id), eq(trainers.isNpc, true)));
    return {};
  },
};
