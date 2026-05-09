import * as server from '../entries/pages/wiki/abilities/_page.server.ts.js';

export const index = 5;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/wiki/abilities/_page.svelte.js')).default;
export { server };
export const server_id = "src/routes/wiki/abilities/+page.server.ts";
export const imports = ["_app/immutable/nodes/5.CWHqmP3_.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js"];
export const stylesheets = ["_app/immutable/assets/5.B20YUcrz.css"];
export const fonts = [];
