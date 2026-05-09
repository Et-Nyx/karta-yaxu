import * as server from '../entries/pages/wiki/types/_page.server.ts.js';

export const index = 11;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/wiki/types/_page.svelte.js')).default;
export { server };
export const server_id = "src/routes/wiki/types/+page.server.ts";
export const imports = ["_app/immutable/nodes/11.D3365xvM.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/C-DBfdJn.js"];
export const stylesheets = ["_app/immutable/assets/TypeIcon.DYCWKU_I.css","_app/immutable/assets/11.CpCjSRVP.css"];
export const fonts = [];
