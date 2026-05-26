import { lucia } from '$lib/auth/lucia';
import type { Handle } from '@sveltejs/kit';
import { auditStore } from '$lib/server/context';

export const handle: Handle = async ({ event, resolve }) => {
  const sessionId = event.cookies.get(lucia.sessionCookieName);

  if (!sessionId) {
    event.locals.user = null;
    event.locals.session = null;
  } else {
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
    } else {
      event.locals.user = user;
      event.locals.session = session;
    }
  }

  // Check for edit/import audit comments in POST requests
  let auditComment: string | null = null;
  if (event.request.method === 'POST') {
    try {
      const cloned = event.request.clone();
      const formData = await cloned.formData();
      const commentVal = formData.get('auditComment');
      if (commentVal !== null) {
        auditComment = String(commentVal).trim() || null;
      }
    } catch {}
  }

  // Wrap request resolution in Node's AsyncLocalStorage context
  return auditStore.run({ comment: auditComment }, () => resolve(event));
};
