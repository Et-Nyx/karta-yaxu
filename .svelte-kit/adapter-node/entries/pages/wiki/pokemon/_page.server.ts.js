import { N as types, m as pokemonTypes, o as pokemon, t as db } from "../../../../chunks/db.js";
import { asc, eq } from "drizzle-orm";
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
//#endregion
export { load };
