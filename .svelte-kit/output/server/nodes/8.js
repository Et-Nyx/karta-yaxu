import * as server from '../entries/pages/wiki/moves/_slug_/_page.server.ts.js';

export const index = 8;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/wiki/moves/_slug_/_page.svelte.js')).default;
export { server };
export const server_id = "src/routes/wiki/moves/[slug]/+page.server.ts";
export const imports = ["_app/immutable/nodes/8.EaS1cuAH.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/Bn8we97R.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js"];
export const stylesheets = ["_app/immutable/assets/MoveCategoryIcon.CLs06Yuq.css","_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/8.Byy4mht9.css"];
export const fonts = [];
