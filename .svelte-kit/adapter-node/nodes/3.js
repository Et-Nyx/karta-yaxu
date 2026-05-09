import * as server from '../entries/pages/import/_page.server.ts.js';

export const index = 3;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/import/_page.svelte.js')).default;
export { server };
export const server_id = "src/routes/import/+page.server.ts";
export const imports = ["_app/immutable/nodes/3.CandOcun.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/DsitodRo.js","_app/immutable/chunks/CP97kCR3.js"];
export const stylesheets = ["_app/immutable/assets/3.DEntuJaD.css"];
export const fonts = [];
