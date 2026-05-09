import { d as db, t as types, j as typeEffectiveness, a as pokemon, b as pokemonTypes } from './db-IHYKL_-q.js';
import { e as error } from './index-CLnuRv4X.js';
import { eq, asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';
import './index-DBqjc0Yf.js';

//#region src/routes/wiki/types/[slug]/+page.server.ts
var load = async ({ params }) => {
	const { slug } = params;
	const tList = await db.select().from(types).where(eq(types.slug, slug)).limit(1);
	if (tList.length === 0) throw error(404, "Type not found");
	const typeData = tList[0];
	const attackingList = await db.select({
		defenderName: types.name,
		multiplier: typeEffectiveness.multiplier
	}).from(typeEffectiveness).innerJoin(types, eq(typeEffectiveness.defenderId, types.id)).where(eq(typeEffectiveness.attackerId, typeData.id));
	const defendingList = await db.select({
		attackerName: types.name,
		multiplier: typeEffectiveness.multiplier
	}).from(typeEffectiveness).innerJoin(types, eq(typeEffectiveness.attackerId, types.id)).where(eq(typeEffectiveness.defenderId, typeData.id));
	const pokemonData = await db.select({
		id: pokemon.id,
		pokeapiId: pokemon.pokeapiId,
		name: pokemon.name,
		slug: pokemon.slug,
		spriteDefault: pokemon.spriteDefault
	}).from(pokemonTypes).innerJoin(pokemon, eq(pokemonTypes.pokemonId, pokemon.id)).where(eq(pokemonTypes.typeId, typeData.id)).orderBy(asc(pokemon.pokeapiId));
	const pIds = pokemonData.map((p) => p.id);
	let typesMap = /* @__PURE__ */ new Map();
	if (pIds.length > 0) {
		const pTypes = await db.select({
			pokemonId: pokemonTypes.pokemonId,
			typeName: types.name
		}).from(pokemonTypes).innerJoin(types, eq(pokemonTypes.typeId, types.id)).orderBy(asc(pokemonTypes.slot));
		for (const pt of pTypes) {
			if (!typesMap.has(pt.pokemonId)) typesMap.set(pt.pokemonId, []);
			typesMap.get(pt.pokemonId).push(pt.typeName);
		}
	}
	return {
		typeData,
		attackingList,
		defendingList,
		pokemonList: pokemonData.map((p) => ({
			...p,
			types: typesMap.get(p.id) || []
		}))
	};
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 12;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-UKDLQyy1.js')).default;
const server_id = "src/routes/wiki/types/[slug]/+page.server.ts";
const imports = ["_app/immutable/nodes/12.ClDgdCKY.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js","_app/immutable/chunks/C-DBfdJn.js"];
const stylesheets = ["_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/TypeIcon.DYCWKU_I.css","_app/immutable/assets/12.CBh7x7b3.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=12-3ou57VoO.js.map
