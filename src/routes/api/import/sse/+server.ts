import type { RequestHandler } from './$types';
import { db } from '$lib/db/index';
import * as schema from '$lib/db/schema/index';
import { gt, asc } from 'drizzle-orm';
import { importEvents } from '$lib/pokeapi/import-events';
import type { ImportLogEvent } from '$lib/pokeapi/import-events';

export const GET: RequestHandler = async ({ request, url }) => {
  let controller: ReadableStreamDefaultController;

  const lastIdHeader = request.headers.get('Last-Event-ID');
  const lastIdParam = url.searchParams.get('lastId');
  const lastId = Number(lastIdHeader || lastIdParam || '0');

  const stream = new ReadableStream({
    async start(c) {
      controller = c;
      
      const onLog = (event: ImportLogEvent) => {
        try {
          const sseMsg = event.id
            ? `id: ${event.id}\ndata: ${JSON.stringify(event)}\n\n`
            : `data: ${JSON.stringify(event)}\n\n`;
          controller.enqueue(sseMsg);
        } catch {
          importEvents.off('log', onLog);
        }
      };

      // 1. Sync missed events if lastId was supplied
      if (lastId > 0) {
        try {
          const missed = await db.select()
            .from(schema.importLog)
            .where(gt(schema.importLog.id, lastId))
            .orderBy(asc(schema.importLog.id));

          for (const m of missed) {
            const event: ImportLogEvent = {
              id: m.id,
              entityType: m.entityType,
              pokeapiId: m.pokeapiId,
              action: m.action as 'inserted' | 'updated' | 'skipped',
              reason: m.reason ?? undefined,
              comment: m.comment,
              timestamp: m.timestamp ? new Date(m.timestamp).toISOString() : new Date().toISOString(),
            };
            controller.enqueue(`id: ${m.id}\ndata: ${JSON.stringify(event)}\n\n`);
          }
        } catch (e) {
          console.error('Failed to stream missed import events:', e);
        }
      }

      // Ping to keep connection alive every 15s
      const pingInterval = setInterval(() => {
        try {
          controller.enqueue(': ping\n\n');
        } catch (e) {
          clearInterval(pingInterval);
        }
      }, 15000);

      // Subscribe to events
      importEvents.on('log', onLog);

      // Cleanup when connection closes
      request.signal.addEventListener('abort', () => {
        clearInterval(pingInterval);
        importEvents.off('log', onLog);
        try {
          controller.close();
        } catch (e) {}
      });
    },
    cancel() {
      // The stream is cancelled by the client
    }
  });

  return new Response(stream, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive',
    }
  });
};
