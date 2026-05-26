import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import { eq } from 'drizzle-orm';
import { requireEditor } from '$lib/auth/guards';

export const POST: RequestHandler = async ({ locals }) => {
  requireEditor(locals);
  await db
    .delete(schema.stagedImports)
    .where(eq(schema.stagedImports.trainerId, locals.user!.id));
  return json({ success: true });
};
