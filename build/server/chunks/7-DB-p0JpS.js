import { d as db, t as types, o as moves } from './db-IHYKL_-q.js';
import { eq, asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';

//#region src/routes/wiki/moves/+page.server.ts
var load = async () => {
	return { moves: await db.select({
		id: moves.id,
		name: moves.name,
		slug: moves.slug,
		power: moves.power,
		accuracy: moves.accuracy,
		pp: moves.pp,
		priority: moves.priority,
		damageClass: moves.damageClass,
		typeName: types.name
	}).from(moves).leftJoin(types, eq(moves.typeId, types.id)).orderBy(asc(moves.id)) };
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 7;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-B3nZfcyJ.js')).default;
const server_id = "src/routes/wiki/moves/+page.server.ts";
const imports = ["_app/immutable/nodes/7.4gK2ShKh.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/Bn8we97R.js","_app/immutable/chunks/BZnoSUU8.js"];
const stylesheets = ["_app/immutable/assets/MoveCategoryIcon.CLs06Yuq.css","_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/7.DjFpEcyP.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=7-DB-p0JpS.js.map
