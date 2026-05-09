import { d as db, k as abilities, c as pokemonAbilities, a as pokemon, t as types, b as pokemonTypes } from './db-IHYKL_-q.js';
import { e as error } from './index-CLnuRv4X.js';
import { eq, asc } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';
import './index-DBqjc0Yf.js';

//#region src/routes/wiki/abilities/[slug]/+page.server.ts
var load = async ({ params }) => {
	const { slug } = params;
	const aList = await db.select().from(abilities).where(eq(abilities.slug, slug)).limit(1);
	if (aList.length === 0) throw error(404, "Ability not found");
	const ability = aList[0];
	const pokemonData = await db.select({
		id: pokemon.id,
		pokeapiId: pokemon.pokeapiId,
		name: pokemon.name,
		slug: pokemon.slug,
		spriteDefault: pokemon.spriteDefault,
		isHidden: pokemonAbilities.isHidden
	}).from(pokemonAbilities).innerJoin(pokemon, eq(pokemonAbilities.pokemonId, pokemon.id)).where(eq(pokemonAbilities.abilityId, ability.id)).orderBy(asc(pokemon.pokeapiId));
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
		ability,
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

const index = 6;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-Bp7-lHMp.js')).default;
const server_id = "src/routes/wiki/abilities/[slug]/+page.server.ts";
const imports = ["_app/immutable/nodes/6.DocMld25.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js"];
const stylesheets = ["_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/6.B9KYhOJN.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=6-i9PY_2iU.js.map
