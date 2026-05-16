import { fail } from '@sveltejs/kit';
import { eq, asc, count, and } from 'drizzle-orm';
import { randomBytes } from 'node:crypto';
import { generateId } from 'lucia';
import { db } from '$lib/db';
import { trainers } from '$lib/db/schema/trainers';
import { requireEditor } from '$lib/auth/guards';
import type { PageServerLoad, Actions } from './$types';

export const load: PageServerLoad = async ({ locals }) => {
  const all = await db.select({
    id:        trainers.id,
    username:  trainers.username,
    role:      trainers.role,
    isNpc:     trainers.isNpc,
    createdAt: trainers.createdAt,
    bannedAt:  trainers.bannedAt,
  }).from(trainers).orderBy(asc(trainers.createdAt));

  return {
    users:         all.filter(t => !t.isNpc),
    npcs:          all.filter(t => t.isNpc),
    isEditor:      locals.user?.role === 'editor',
    currentUserId: locals.user?.id ?? null,
  };
};

export const actions: Actions = {
  promote: async ({ request, locals }) => {
    requireEditor(locals);
    const id = (await request.formData()).get('id') as string;
    if (!id || id === locals.user!.id) return fail(400, { error: 'Ação inválida' });

    const [target] = await db.select({ role: trainers.role, isNpc: trainers.isNpc })
      .from(trainers).where(eq(trainers.id, id)).limit(1);
    if (!target || target.isNpc || target.role === 'editor') return fail(400, { error: 'Ação inválida' });

    await db.update(trainers).set({ role: 'editor' }).where(eq(trainers.id, id));
    return {};
  },

  demote: async ({ request, locals }) => {
    requireEditor(locals);
    const id = (await request.formData()).get('id') as string;
    if (!id || id === locals.user!.id) return fail(400, { error: 'Não é possível rebaixar a si mesmo' });

    const [target] = await db.select({ role: trainers.role, isNpc: trainers.isNpc })
      .from(trainers).where(eq(trainers.id, id)).limit(1);
    if (!target || target.isNpc || target.role !== 'editor') return fail(400, { error: 'Ação inválida' });

    const [{ editorCount }] = await db.select({ editorCount: count() })
      .from(trainers)
      .where(and(eq(trainers.role, 'editor'), eq(trainers.isNpc, false)));
    if (editorCount <= 1) return fail(400, { error: 'Não é possível remover o último editor' });

    await db.update(trainers).set({ role: 'viewer' }).where(eq(trainers.id, id));
    return {};
  },

  ban: async ({ request, locals }) => {
    requireEditor(locals);
    const id = (await request.formData()).get('id') as string;
    if (!id || id === locals.user!.id) return fail(400, { error: 'Não é possível banir a si mesmo' });

    await db.update(trainers).set({ bannedAt: new Date() }).where(eq(trainers.id, id));
    return {};
  },

  unban: async ({ request, locals }) => {
    requireEditor(locals);
    const id = (await request.formData()).get('id') as string;
    if (!id) return fail(400, { error: 'Ação inválida' });

    await db.update(trainers).set({ bannedAt: null }).where(eq(trainers.id, id));
    return {};
  },

  createNpc: async ({ request, locals }) => {
    requireEditor(locals);
    const username = ((await request.formData()).get('username') as string)?.trim();
    if (!username || username.length < 1 || username.length > 30) {
      return fail(400, { createError: 'Nome inválido (1–30 caracteres)' });
    }

    const [existing] = await db.select({ id: trainers.id })
      .from(trainers).where(eq(trainers.username, username)).limit(1);
    if (existing) return fail(400, { createError: 'Nome já em uso' });

    const passwordHash = randomBytes(32).toString('hex') + '.' + randomBytes(16).toString('hex');
    await db.insert(trainers).values({
      id: generateId(15),
      username,
      passwordHash,
      role: 'viewer',
      isNpc: true,
    });
    return {};
  },

  deleteNpc: async ({ request, locals }) => {
    requireEditor(locals);
    const id = (await request.formData()).get('id') as string;
    if (!id) return fail(400, { error: 'Ação inválida' });

    await db.delete(trainers).where(and(eq(trainers.id, id), eq(trainers.isNpc, true)));
    return {};
  },
};
