import { d as db, a as pokemon, p as pokemonSpecies, b as pokemonTypes, t as types, c as pokemonAbilities, k as abilities, f as pokemonStats, e as evolutionDetails, v as versionGroups, O as pokemonFlavorTexts, y as languages, g as pokemonMoves, u as moveLearnMethods, o as moves } from './db-IHYKL_-q.js';
import { e as error } from './index-CLnuRv4X.js';
import { eq, asc, and, inArray } from 'drizzle-orm';
import './chunk-BBx_TEkp.js';
import 'drizzle-orm/better-sqlite3';
import 'better-sqlite3';
import 'drizzle-orm/sqlite-core';
import './index-DBqjc0Yf.js';

//#region src/routes/wiki/pokemon/[slug]/+page.server.ts
var load = async ({ params }) => {
	const { slug } = params;
	const pList = await db.select().from(pokemon).where(eq(pokemon.slug, slug)).limit(1);
	if (pList.length === 0) throw error(404, "Pokémon not found");
	const pokemon$1 = pList[0];
	const species = (await db.select().from(pokemonSpecies).where(eq(pokemonSpecies.id, pokemon$1.speciesId)).limit(1))[0];
	const typesData = await db.select({
		name: types.name,
		slot: pokemonTypes.slot
	}).from(pokemonTypes).innerJoin(types, eq(pokemonTypes.typeId, types.id)).where(eq(pokemonTypes.pokemonId, pokemon$1.id)).orderBy(asc(pokemonTypes.slot));
	const abilitiesData = await db.select({
		name: abilities.name,
		slug: abilities.slug,
		isHidden: pokemonAbilities.isHidden,
		slot: pokemonAbilities.slot
	}).from(pokemonAbilities).innerJoin(abilities, eq(pokemonAbilities.abilityId, abilities.id)).where(eq(pokemonAbilities.pokemonId, pokemon$1.id)).orderBy(asc(pokemonAbilities.slot));
	const statsData = await db.select({
		statName: pokemonStats.statName,
		baseStat: pokemonStats.baseStat,
		effort: pokemonStats.effort
	}).from(pokemonStats).where(eq(pokemonStats.pokemonId, pokemon$1.id));
	const statOrder = [
		"hp",
		"attack",
		"defense",
		"special-attack",
		"special-defense",
		"speed"
	];
	statsData.sort((a, b) => statOrder.indexOf(a.statName) - statOrder.indexOf(b.statName));
	let evolutionNodes = [];
	let evolutionEdges = [];
	if (species.evolutionChainId) {
		evolutionEdges = await db.select().from(evolutionDetails).where(eq(evolutionDetails.chainId, species.evolutionChainId));
		evolutionNodes = await db.select({
			speciesId: pokemonSpecies.id,
			pokeapiId: pokemon.pokeapiId,
			name: pokemon.name,
			slug: pokemon.slug,
			spriteDefault: pokemon.spriteDefault,
			isBaby: pokemonSpecies.isBaby,
			order: pokemonSpecies.order
		}).from(pokemonSpecies).innerJoin(pokemon, eq(pokemonSpecies.id, pokemon.speciesId)).where(and(eq(pokemonSpecies.evolutionChainId, species.evolutionChainId), eq(pokemon.isDefault, true)));
		if (evolutionNodes.length > 0) {
			const nodeIds = evolutionNodes.map((n) => n.speciesId).filter(Boolean);
			const evoTypes = await db.select({
				speciesId: pokemonSpecies.id,
				typeName: types.name
			}).from(pokemonTypes).innerJoin(pokemon, eq(pokemonTypes.pokemonId, pokemon.id)).innerJoin(pokemonSpecies, eq(pokemon.speciesId, pokemonSpecies.id)).innerJoin(types, eq(pokemonTypes.typeId, types.id)).where(inArray(pokemonSpecies.id, nodeIds)).orderBy(asc(pokemonTypes.slot));
			const typesMap = /* @__PURE__ */ new Map();
			for (const et of evoTypes) {
				if (!typesMap.has(et.speciesId)) typesMap.set(et.speciesId, []);
				typesMap.get(et.speciesId).push(et.typeName);
			}
			evolutionNodes = evolutionNodes.map((n) => ({
				...n,
				types: typesMap.get(n.speciesId) || []
			}));
		}
	}
	const availableVersions = await db.select({
		id: versionGroups.id,
		name: versionGroups.name,
		order: versionGroups.order
	}).from(versionGroups).orderBy(asc(versionGroups.order));
	const flavorTexts = await db.select({
		flavorText: pokemonFlavorTexts.flavorText,
		versionGroupId: pokemonFlavorTexts.versionGroupId,
		languageId: pokemonFlavorTexts.languageId
	}).from(pokemonFlavorTexts).innerJoin(languages, eq(pokemonFlavorTexts.languageId, languages.id)).where(and(eq(pokemonFlavorTexts.speciesId, species.id), eq(languages.name, "en")));
	const learnset = await db.select({
		moveName: moves.name,
		moveSlug: moves.slug,
		damageClass: moves.damageClass,
		typeName: types.name,
		level: pokemonMoves.level,
		methodName: moveLearnMethods.name,
		versionGroupId: pokemonMoves.versionGroupId
	}).from(pokemonMoves).innerJoin(moves, eq(pokemonMoves.moveId, moves.id)).innerJoin(types, eq(moves.typeId, types.id)).innerJoin(moveLearnMethods, eq(pokemonMoves.learnMethodId, moveLearnMethods.id)).where(eq(pokemonMoves.pokemonId, pokemon$1.id)).orderBy(asc(pokemonMoves.level));
	return {
		pokemon: pokemon$1,
		species,
		types: typesData,
		abilities: abilitiesData,
		stats: statsData,
		evolutionNodes,
		evolutionEdges,
		availableVersions,
		flavorTexts,
		learnset
	};
};

var _page_server_ts = /*#__PURE__*/Object.freeze({
	__proto__: null,
	load: load
});

const index = 10;
let component_cache;
const component = async () => component_cache ??= (await import('./_page.svelte-BiIhYeVx.js')).default;
const server_id = "src/routes/wiki/pokemon/[slug]/+page.server.ts";
const imports = ["_app/immutable/nodes/10.ruccuHgK.js","_app/immutable/chunks/DbrZRxea.js","_app/immutable/chunks/CP97kCR3.js","_app/immutable/chunks/Bn8we97R.js","_app/immutable/chunks/DAmH0MAf.js","_app/immutable/chunks/BZnoSUU8.js"];
const stylesheets = ["_app/immutable/assets/MoveCategoryIcon.CLs06Yuq.css","_app/immutable/assets/TypeBadge.DZwSkmhI.css","_app/immutable/assets/PokemonCard.BYp4eU5k.css","_app/immutable/assets/10.zlAts7De.css"];
const fonts = [];

export { component, fonts, imports, index, _page_server_ts as server, server_id, stylesheets };
//# sourceMappingURL=10-_auNWfkl.js.map
