import { N as types, t as db } from "../../../../chunks/db.js";
import { asc } from "drizzle-orm";
//#region src/routes/wiki/types/+page.server.ts
var load = async () => {
	return { types: await db.select({
		id: types.id,
		name: types.name,
		slug: types.slug,
		iconUrl: types.iconUrl
	}).from(types).orderBy(asc(types.id)) };
};
//#endregion
export { load };
