import { db } from '$lib/db';
import { edits } from '$lib/db/schema/trainers';
import { auditStore } from '$lib/server/context';

export async function recordEdit(
  trainerId: string,
  tableName: string,
  recordId: number,
  previousData: unknown | null,
  newData: unknown | null,
): Promise<void> {
  const context = auditStore.getStore();
  const comment = context?.comment ?? null;

  await db.insert(edits).values({
    trainerId,
    tableName,
    recordId,
    previousData: previousData == null ? null : JSON.stringify(previousData),
    newData:      newData      == null ? null : JSON.stringify(newData),
    comment,
  });
}
