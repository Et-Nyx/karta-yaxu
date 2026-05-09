import { d as db, a as pokemon, b as pokemonTypes, t as types } from './db-IHYKL_-q.js';
import { eq, asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';

//#region src/routes/wiki/pokemon/+page.server.ts
var load = async () => {
	const pokemonList = await db.select({
		id: pokemon.id,
		pokeapiId: pokemon.pokeapiId,
		name: pokemon.name,
		slug: pokemon.slug,
		spriteDefault: pokemon.spriteDefault,
		order: pokemon.order
	}).from(pokemon).where(eq(pokemon.isDefault, true)).orderBy(asc(pokemon.order)).limit(1010);
	const pokemonTypesList = await db.select({
		pokemonId: pokemonTypes.pokemonId,
		typeName: types.name,
		slot: pokemonTypes.slot
	}).from(pokemonTypes).innerJoin(types, eq(pokemonTypes.typeId, types.id)).orderBy(asc(pokemonTypes.slot));
	const typesMap = /* @__PURE__ */ new Map();
	for (const pt of pokemonTypesList) {
		if (!typesMap.has(pt.pokemonId)) typesMap.set(pt.pokemonId, []);
		typesMap.get(pt.pokemonId).push(pt.typeName);
	}
	return { pokemon: pokemonList.map((p) => ({
		...p,
		types: typesMap.get(p.id) || []
	})) };
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 9;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-Dsm0D_Y9.js')).default;
const server_id = "src/routes/wiki/pokemon/+page.server.ts";
const imports = ["_app/immutable/nodes/9.DLDptBKf.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js"];
const stylesheets = ["_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/9.DeaFIFYj.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=9-B8eoK9o5.js.map
