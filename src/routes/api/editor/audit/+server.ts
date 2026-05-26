import { json } from '@sveltejs/kit';
import { db } from '$lib/db';
import { importLog, edits } from '$lib/db/schema/trainers';
import { eq } from 'drizzle-orm';
import { requireEditor } from '$lib/auth/guards';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, locals }) => {
  requireEditor(locals);

  try {
    const { type, id, comment } = await request.json();

    if (!type || !id || typeof comment !== 'string') {
      return json({ error: 'Invalid request parameters' }, { status: 400 });
    }

    const cleanComment = comment.trim();

    if (type === 'import') {
      await db.update(importLog)
        .set({ comment: cleanComment || null })
        .where(eq(importLog.id, Number(id)));
    } else if (type === 'edit') {
      await db.update(edits)
        .set({ comment: cleanComment || null })
        .where(eq(edits.id, Number(id)));
    } else {
      return json({ error: 'Invalid audit type' }, { status: 400 });
    }

    return json({ success: true });
  } catch (e: any) {
    return json({ error: e.message || 'Server error' }, { status: 500 });
  }
};
