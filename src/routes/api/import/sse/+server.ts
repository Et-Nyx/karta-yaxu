import type { RequestHandler } from './$types';
import { importEvents } from '$lib/pokeapi/import-events';
import type { ImportLogEvent } from '$lib/pokeapi/import-events';

export const GET: RequestHandler = ({ request }) => {
  let controller: ReadableStreamDefaultController;

  const stream = new ReadableStream({
    start(c) {
      controller = c;
      
      const onLog = (event: ImportLogEvent) => {
        try {
          const payload = `data: ${JSON.stringify(event)}\n\n`;
          controller.enqueue(payload);
        } catch (e) {
          console.error('Failed to enqueue SSE log', e);
        }
      };

      // Ping to keep connection alive every 15s (important for proxies/routers)
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
