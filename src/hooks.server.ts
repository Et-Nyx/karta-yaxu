import { lucia } from '$lib/auth/lucia';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  const sessionId = event.cookies.get(lucia.sessionCookieName);

  if (!sessionId) {
    event.locals.user = null;
    event.locals.session = null;
    return resolve(event);
  }

  const { session, user } = await lucia.validateSession(sessionId);

  if (session?.fresh) {
    const cookie = lucia.createSessionCookie(session.id);
    event.cookies.set(cookie.name, cookie.value, { path: '/', ...cookie.attributes });
  }
  if (!session) {
    const cookie = lucia.createBlankSessionCookie();
    event.cookies.set(cookie.name, cookie.value, { path: '/', ...cookie.attributes });
  }

  if (session && user?.bannedAt) {
    await lucia.invalidateSession(session.id);
    const blank = lucia.createBlankSessionCookie();
    event.cookies.set(blank.name, blank.value, { path: '/', ...blank.attributes });
    event.locals.user = null;
    event.locals.session = null;
    return resolve(event);
  }

  event.locals.user = user;
  event.locals.session = session;
  return resolve(event);
};
