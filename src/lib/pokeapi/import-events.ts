import { EventEmitter } from 'node:events';

// Create a singleton global EventEmitter specifically for import logs.
// We attach it to globalThis to ensure it persists across HMR (Hot Module Replacement)
// during development in SvelteKit.
const EMITTER_KEY = Symbol.for('karta_yaxu_import_emitter');

if (!(globalThis as any)[EMITTER_KEY]) {
  (globalThis as any)[EMITTER_KEY] = new EventEmitter();
}

export const importEvents = (globalThis as any)[EMITTER_KEY] as EventEmitter;

export interface ImportLogEvent {
  entityType: string;
  pokeapiId: number;
  action: 'inserted' | 'updated' | 'skipped';
  reason?: string;
  timestamp: string; // ISO string
}
