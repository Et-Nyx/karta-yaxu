import * as server from '../entries/pages/wiki/types/_slug_/_page.server.ts.js';

export const index = 12;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/wiki/types/_slug_/_page.svelte.js')).default;
export { server };
export const server_id = "src/routes/wiki/types/[slug]/+page.server.ts";
export const imports = ["_app/immutable/nodes/12.ClDgdCKY.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js","_app/immutable/chunks/C-DBfdJn.js"];
export const stylesheets = ["_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/TypeIcon.DYCWKU_I.css","_app/immutable/assets/12.CBh7x7b3.css"];
export const fonts = [];
