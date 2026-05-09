import { d as db, t as types, o as moves, a as pokemon, g as pokemonMoves, b as pokemonTypes } from './db-IHYKL_-q.js';
import { e as error } from './index-CLnuRv4X.js';
import { eq, asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';
import './index-DBqjc0Yf.js';

//#region src/routes/wiki/moves/[slug]/+page.server.ts
var load = async ({ params }) => {
	const { slug } = params;
	const mList = await db.select({
		id: moves.id,
		name: moves.name,
		power: moves.power,
		accuracy: moves.accuracy,
		pp: moves.pp,
		damageClass: moves.damageClass,
		priority: moves.priority,
		target: moves.target,
		typeName: types.name
	}).from(moves).leftJoin(types, eq(moves.typeId, types.id)).where(eq(moves.slug, slug)).limit(1);
	if (mList.length === 0) throw error(404, "Move not found");
	const move = mList[0];
	const pokemonLearnersData = await db.select({
		id: pokemon.id,
		pokeapiId: pokemon.pokeapiId,
		name: pokemon.name,
		slug: pokemon.slug,
		spriteDefault: pokemon.spriteDefault
	}).from(pokemonMoves).innerJoin(pokemon, eq(pokemonMoves.pokemonId, pokemon.id)).where(eq(pokemonMoves.moveId, move.id)).groupBy(pokemon.id).orderBy(asc(pokemon.pokeapiId));
	const learnerIds = pokemonLearnersData.map((p) => p.id);
	let typesMap = /* @__PURE__ */ new Map();
	if (learnerIds.length > 0) {
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
		move,
		learners: pokemonLearnersData.map((p) => ({
			...p,
			types: typesMap.get(p.id) || []
		}))
	};
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 8;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-CtqbJaZz.js')).default;
const server_id = "src/routes/wiki/moves/[slug]/+page.server.ts";
const imports = ["_app/immutable/nodes/8.EaS1cuAH.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/Bn8we97R.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js"];
const stylesheets = ["_app/immutable/assets/MoveCategoryIcon.CLs06Yuq.css","_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/8.Byy4mht9.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=8-CdBisHVb.js.map
