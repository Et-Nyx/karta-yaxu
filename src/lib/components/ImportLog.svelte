<script lang="ts">
  export type LogEntry = {
    id: number;
    entityType: string;
    pokeapiId: number;
    action: 'inserted' | 'updated' | 'skipped';
    reason: string | null;
    timestamp: Date | null;
  };

  let { logs = [] as LogEntry[] } = $props();

  function formatTime(date: Date | null): string {
    if (!date) return '';
    return new Date(date).toLocaleTimeString('en-US', { hour12: false });
  }
</script>

<div class="log-container">
  <div class="log-header">
    <h3>IMPORT LOG (LATEST 50)</h3>
  </div>
  
  <div class="log-body">
    {#if logs.length === 0}
      <div class="empty-state">No import activity recorded yet.</div>
    {:else}
      <table class="log-table">
        <thead>
          <tr>
            <th>TIME</th>
            <th>ENTITY</th>
            <th>ID</th>
            <th>ACTION</th>
            <th>REASON</th>
          </tr>
        </thead>
        <tbody>
          {#each logs as log}
            <tr class="action-{log.action}">
              <td class="time">{formatTime(log.timestamp)}</td>
              <td class="entity">{log.entityType}</td>
              <td class="id">#{log.pokeapiId}</td>
              <td class="action">
                <span class="badge {log.action}">{log.action.toUpperCase()}</span>
              </td>
              <td class="reason">{log.reason || '-'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
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
    height: 400px;
  }

  .log-header {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 4px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
  }

  h3 {
    color: var(--theme-text-inverse);
    font-family: var(--font-display);
    font-size: 10px;
    margin: 0;
  }

  .log-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-secondary);
  }

  .empty-state {
    font-family: var(--font-mono);
    color: var(--theme-text-muted);
    padding: var(--spacing-4);
    text-align: center;
    font-size: 12px;
  }

  .log-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 11px;
    text-align: left;
  }

  th {
    position: sticky;
    top: 0;
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-secondary);
    padding: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
    font-weight: bold;
  }

  td {
    padding: var(--spacing-2);
    border-bottom: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }

  tr:hover td {
    background-color: var(--theme-bg-hover);
  }

  .time { color: var(--theme-text-muted); }
  .entity { font-weight: bold; }
  .id { color: var(--theme-text-secondary); }
  
  .badge {
    padding: 2px 4px;
    font-family: var(--font-display);
    font-size: 8px;
  }

  .badge.inserted { background-color: var(--theme-text-primary); color: var(--theme-bg-secondary); }
  .badge.updated { background-color: var(--color-text-warning); color: #000; }
  .badge.skipped { background-color: var(--theme-text-muted); color: var(--theme-bg-primary); }
  
  .reason { color: var(--theme-text-secondary); font-size: 10px; }
</style>
