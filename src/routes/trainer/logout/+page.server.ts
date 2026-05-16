import { redirect } from '@sveltejs/kit';
import { lucia } from '$lib/auth/lucia';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ locals, cookies }) => {
    if (locals.session) {
      await lucia.invalidateSession(locals.session.id);
    }
    const cookie = lucia.createBlankSessionCookie();
    cookies.set(cookie.name, cookie.value, { path: '/', ...cookie.attributes });
    throw redirect(302, '/');
  },
};
