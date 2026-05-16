import { z } from 'zod';
import { fail, redirect } from '@sveltejs/kit';
import { eq, count } from 'drizzle-orm';
import { generateId } from 'lucia';
import { db } from '$lib/db';
import { trainers } from '$lib/db/schema/trainers';
import { lucia } from '$lib/auth/lucia';
import { hashPassword } from '$lib/auth/password';
import type { Actions } from './$types';

const Schema = z.object({
  username: z.string().min(3, 'Mínimo 3 caracteres').max(20, 'Máximo 20 caracteres')
    .regex(/^[a-zA-Z0-9_]+$/, 'Apenas letras, números e _'),
  password: z.string().min(6, 'Mínimo 6 caracteres'),
  confirm:  z.string(),
}).refine(d => d.password === d.confirm, {
  message: 'Senhas não coincidem',
  path: ['confirm'],
});

export const actions: Actions = {
  default: async ({ request, cookies }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = Schema.safeParse(form);

    if (!parsed.success) {
      return fail(400, { errors: parsed.error.flatten().fieldErrors });
    }

    const { username, password } = parsed.data;

    const existing = await db.select({ id: trainers.id })
      .from(trainers).where(eq(trainers.username, username)).limit(1);
    if (existing.length > 0) {
      return fail(400, { errors: { username: ['Username já em uso'] } });
    }

    const [{ total }] = await db.select({ total: count() }).from(trainers);
    const role: 'editor' | 'viewer' = total === 0 ? 'editor' : 'viewer';

    const id = generateId(15);
    const passwordHash = await hashPassword(password);

    await db.insert(trainers).values({ id, username, passwordHash, role });

    const session = await lucia.createSession(id, {});
    const cookie = lucia.createSessionCookie(session.id);
    cookies.set(cookie.name, cookie.value, { path: '/', ...cookie.attributes });

    throw redirect(302, role === 'editor' ? '/import' : '/wiki');
  },
};
