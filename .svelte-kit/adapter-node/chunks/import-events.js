import { EventEmitter } from "node:events";
//#region src/lib/pokeapi/import-events.ts
var EMITTER_KEY = Symbol.for("karta_yaxu_import_emitter");
if (!globalThis[EMITTER_KEY]) globalThis[EMITTER_KEY] = new EventEmitter();
var importEvents = globalThis[EMITTER_KEY];
//#endregion
export { importEvents as t };
