<script module lang="ts">
  export type LogEntry = {
    id: number;
    entityType: string;
    pokeapiId: number;
    action: 'inserted' | 'updated' | 'skipped';
    reason: string | null;
    comment: string | null;
    timestamp: Date | null;
  };
</script>

<script lang="ts">
  import DataTable from './DataTable.svelte';

  let { logs = [] as LogEntry[], onRowClick } = $props<{
    logs: LogEntry[];
    onRowClick?: (log: LogEntry) => void;
  }>();

  function formatTime(date: Date | null): string {
    if (!date) return '';
    return new Date(date).toLocaleTimeString('en-US', { hour12: false });
  }
</script>

<div class="log-container">
  <div class="log-body">
    {#if logs.length === 0}
      <div class="empty-state">No import activity recorded yet.</div>
    {:else}
      {#snippet head()}
        <th>TIME</th>
        <th>ENTITY</th>
        <th>ID</th>
        <th>ACTION</th>
        <th>REASON</th>
      {/snippet}

      {#snippet tableBody()}
        {#each logs as log}
          <tr class="action-{log.action} clickable-row" onclick={() => onRowClick?.(log)}>
            <td class="time">{formatTime(log.timestamp)}</td>
            <td class="entity">{log.entityType.toUpperCase()}</td>
            <td class="id">#{log.pokeapiId}</td>
            <td class="action">
              <span class="badge {log.action}">{log.action.toUpperCase()}</span>
            </td>
            <td class="reason">
              {log.reason || '—'}
              {#if log.comment}
                <span class="comment-icon" title={log.comment}>💬</span>
              {/if}
            </td>
          </tr>
        {/each}
      {/snippet}

      <DataTable head={head} children={tableBody} compact={true} />
    {/if}
  </div>
</div>

<style>
  .log-container {
    background-color: var(--theme-bg-primary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    display: flex;
    flex-direction: column;
    height: 480px;
  }

  .log-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    padding: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
  }

  .empty-state {
    font-family: var(--font-mono);
    color: var(--theme-text-muted);
    padding: var(--spacing-4);
    text-align: center;
    font-size: 12px;
  }

  .clickable-row {
    cursor: pointer;
    transition: background-color 0.1s ease;
  }

  .clickable-row:hover {
    background-color: rgba(0, 255, 0, 0.05) !important;
  }

  .time { color: var(--theme-text-muted); }
  .entity { font-weight: bold; }
  .id { color: var(--theme-text-secondary); }

  .badge {
    padding: 2px 6px;
    font-family: var(--font-display);
    font-size: 8px;
  }

  .badge.inserted { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }
  .badge.updated { background-color: var(--color-text-warning, #d4a017); color: #000; }
  .badge.skipped { background-color: var(--theme-text-muted); color: var(--theme-bg-primary); }

  .reason { 
    color: var(--theme-text-secondary); 
    font-size: 10px;
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .comment-icon {
    font-size: 10px;
    cursor: help;
  }
</style>
