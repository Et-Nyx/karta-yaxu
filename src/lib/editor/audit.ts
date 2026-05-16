import { db } from '$lib/db';
import { edits } from '$lib/db/schema/trainers';

export async function recordEdit(
  trainerId: string,
  tableName: string,
  recordId: number,
  previousData: unknown | null,
  newData: unknown | null,
): Promise<void> {
  await db.insert(edits).values({
    trainerId,
    tableName,
    recordId,
    previousData: previousData == null ? null : JSON.stringify(previousData),
    newData:      newData      == null ? null : JSON.stringify(newData),
  });
}
