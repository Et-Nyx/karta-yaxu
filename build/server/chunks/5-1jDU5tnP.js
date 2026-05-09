import { d as db, k as abilities } from './db-IHYKL_-q.js';
import { asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';

//#region src/routes/wiki/abilities/+page.server.ts
var load = async () => {
	return { abilities: await db.select().from(abilities).orderBy(asc(abilities.id)) };
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 5;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-DCusMaHU.js')).default;
const server_id = "src/routes/wiki/abilities/+page.server.ts";
const imports = ["_app/immutable/nodes/5.CWHqmP3_.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js"];
const stylesheets = ["_app/immutable/assets/5.B20YUcrz.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=5-1jDU5tnP.js.map
