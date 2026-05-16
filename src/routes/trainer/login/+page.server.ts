import { z } from 'zod';
import { fail, redirect } from '@sveltejs/kit';
import { eq } from 'drizzle-orm';
import { db } from '$lib/db';
import { trainers } from '$lib/db/schema/trainers';
import { lucia } from '$lib/auth/lucia';
import { verifyPassword } from '$lib/auth/password';
import type { Actions } from './$types';

const Schema = z.object({
  username: z.string().min(1),
  password: z.string().min(1),
});

export const actions: Actions = {
  default: async ({ request, cookies }) => {
    const form = Object.fromEntries(await request.formData());
    const parsed = Schema.safeParse(form);

    if (!parsed.success) {
      return fail(400, { error: 'Preencha todos os campos' });
    }

    const { username, password } = parsed.data;

    const [trainer] = await db.select().from(trainers)
      .where(eq(trainers.username, username)).limit(1);

    const valid = trainer && !trainer.isNpc && await verifyPassword(trainer.passwordHash, password);
    if (!valid) {
      return fail(400, { error: 'Username ou senha incorretos' });
    }

    if (trainer.bannedAt) {
      return fail(403, { error: 'Esta conta foi banida' });
    }

    const session = await lucia.createSession(trainer.id, {});
    const cookie = lucia.createSessionCookie(session.id);
    cookies.set(cookie.name, cookie.value, { path: '/', ...cookie.attributes });

    throw redirect(302, trainer.role === 'editor' ? '/import' : '/wiki');
  },
};
