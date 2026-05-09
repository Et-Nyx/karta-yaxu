import { N as types, m as pokemonTypes, o as pokemon, s as pokemonAbilities, t as db, x as abilities } from "../../../../../chunks/db.js";
import { error } from "@sveltejs/kit";
import { asc, eq } from "drizzle-orm";
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
//#endregion
export { load };
