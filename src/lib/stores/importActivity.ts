import { writable } from 'svelte/store';

export interface ImportActivityState {
  isActive: boolean;
  entityType: string | null;
  inserted: number;
  updated: number;
  skipped: number;
}

const IDLE: ImportActivityState = {
  isActive: false,
  entityType: null,
  inserted: 0,
  updated: 0,
  skipped: 0,
};

export const importActivity = writable<ImportActivityState>({ ...IDLE });

let idleTimer: ReturnType<typeof setTimeout> | null = null;
let clearTimer: ReturnType<typeof setTimeout> | null = null;

function clearTimers() {
  if (idleTimer) { clearTimeout(idleTimer); idleTimer = null; }
  if (clearTimer) { clearTimeout(clearTimer); clearTimer = null; }
}

// Call this the moment the user clicks an import button — shows indicator immediately
// before the first SSE event arrives.
export function startImport() {
  clearTimers();
  importActivity.update(s => ({ ...s, isActive: true }));
}

export function handleImportEvent(event: {
  entityType: string;
  action: string;
  pokeapiId: number;
}) {
  clearTimers();

  importActivity.update(s => {
    const isNewBatch = s.entityType !== null && s.entityType !== event.entityType;
    const base = isNewBatch ? { ...IDLE } : { ...s };
    return {
      ...base,
      isActive: true,
      entityType: event.entityType,
      inserted: base.inserted + (event.action === 'inserted' ? 1 : 0),
      updated:  base.updated  + (event.action === 'updated'  ? 1 : 0),
      skipped:  base.skipped  + (event.action === 'skipped'  ? 1 : 0),
    };
  });

  // 10s after last event → mark done (keep counts visible)
  idleTimer = setTimeout(() => {
    importActivity.update(s => ({ ...s, isActive: false }));
    // 10s after done → clear entirely
    clearTimer = setTimeout(() => importActivity.set({ ...IDLE }), 10000);
  }, 10000);
}

export interface StagingState {
  entityType: string | null;
  fetched: number;
  failed: number;
  total: number;
  status: 'idle' | 'fetching' | 'fetched' | 'failed' | 'migrating';
  activeName?: string | null;
  activeId?: number | null;
}

export const stagingActivity = writable<StagingState>({
  entityType: null,
  fetched: 0,
  failed: 0,
  total: 0,
  status: 'idle',
  activeName: null,
  activeId: null,
});
