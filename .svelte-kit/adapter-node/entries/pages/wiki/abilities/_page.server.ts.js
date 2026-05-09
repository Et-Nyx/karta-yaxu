import { t as db, x as abilities } from "../../../../chunks/db.js";
import { asc } from "drizzle-orm";
//#region src/routes/wiki/abilities/+page.server.ts
var load = async () => {
	return { abilities: await db.select().from(abilities).orderBy(asc(abilities.id)) };
};
//#endregion
export { load };
