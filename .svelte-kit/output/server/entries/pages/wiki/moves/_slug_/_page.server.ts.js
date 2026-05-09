import { A as moves, N as types, d as pokemonMoves, m as pokemonTypes, o as pokemon, t as db } from "../../../../../chunks/db.js";
import { error } from "@sveltejs/kit";
import { asc, eq } from "drizzle-orm";
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
//#endregion
export { load };
