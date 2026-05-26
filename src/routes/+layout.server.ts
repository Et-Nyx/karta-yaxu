import type { LayoutServerLoad } from './$types';
import { db } from '$lib/db';
import * as schema from '$lib/db/schema';
import { eq } from 'drizzle-orm';

export const load: LayoutServerLoad = async ({ locals }) => {
  let activeStaged = null;

  if (locals.user) {
    const staged = await db
      .select()
      .from(schema.stagedImports)
      .where(eq(schema.stagedImports.trainerId, locals.user.id));

    if (staged.length > 0) {
      const fetchedCount = staged.filter((s) => s.status === 'fetched').length;
      const failedCount = staged.filter((s) => s.status === 'failed').length;
      const isFetching = staged.some((s) => s.status === 'fetching' || s.status === 'pending');
      const isFailed = staged.some((s) => s.status === 'failed');

      activeStaged = {
        entityType: staged[0].entityType,
        fetched: fetchedCount,
        failed: failedCount,
        total: staged.length,
        status: isFailed ? 'failed' : isFetching ? 'fetching' : 'fetched',
      };
    }
  }

  return {
    user: locals.user,
    activeStaged,
  };
};
