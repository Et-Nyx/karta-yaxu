import { A as moves, N as types, t as db } from "../../../../chunks/db.js";
import { asc, eq } from "drizzle-orm";
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
//#endregion
export { load };
