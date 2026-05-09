import { N as types, j as typeEffectiveness, m as pokemonTypes, o as pokemon, t as db } from "../../../../../chunks/db.js";
import { error } from "@sveltejs/kit";
import { asc, eq } from "drizzle-orm";
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
//#endregion
export { load };
