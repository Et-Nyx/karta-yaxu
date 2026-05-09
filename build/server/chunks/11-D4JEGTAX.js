import { d as db, t as types } from './db-IHYKL_-q.js';
import { asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';

//#region src/routes/wiki/types/+page.server.ts
var load = async () => {
	return { types: await db.select({
		id: types.id,
		name: types.name,
		slug: types.slug,
		iconUrl: types.iconUrl
	}).from(types).orderBy(asc(types.id)) };
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 11;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-BTZ1VLAp.js')).default;
const server_id = "src/routes/wiki/types/+page.server.ts";
const imports = ["_app/immutable/nodes/11.D3365xvM.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/C-DBfdJn.js"];
const stylesheets = ["_app/immutable/assets/TypeIcon.DYCWKU_I.css","_app/immutable/assets/11.CpCjSRVP.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=11-D4JEGTAX.js.map
