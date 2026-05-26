import { AsyncLocalStorage } from 'node:async_hooks';

export interface AuditContext {
  comment?: string | null;
}

export const auditStore = new AsyncLocalStorage<AuditContext>();
