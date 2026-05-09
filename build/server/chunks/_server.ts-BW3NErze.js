import { i as importEvents } from './import-events-CiRDkSb8.js';
import 'node:events';

//#region src/routes/api/import/sse/+server.ts
var GET = ({ request }) => {
	let controller;
	const stream = new ReadableStream({
		start(c) {
			controller = c;
			const onLog = (event) => {
				try {
					const payload = `data: ${JSON.stringify(event)}\n\n`;
					controller.enqueue(payload);
				} catch (e) {
					console.error("Failed to enqueue SSE log", e);
				}
			};
			const pingInterval = setInterval(() => {
				try {
					controller.enqueue(": ping\n\n");
				} catch (e) {
					clearInterval(pingInterval);
				}
			}, 15e3);
			importEvents.on("log", onLog);
			request.signal.addEventListener("abort", () => {
				clearInterval(pingInterval);
				importEvents.off("log", onLog);
				try {
					controller.close();
				} catch (e) {}
			});
		},
		cancel() {}
	});
	return new Response(stream, { headers: {
		"Content-Type": "text/event-stream",
		"Cache-Control": "no-cache",
		"Connection": "keep-alive"
	} });
};

export { GET };
//# sourceMappingURL=_server.ts-BW3NErze.js.map
