<script lang="ts">
  let { data } = $props();

  function formatDate(ts: Date | null): string {
    if (!ts) return '—';
    return new Date(ts).toLocaleDateString('pt-BR', {
      day: '2-digit', month: '2-digit', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<svelte:head>
  <title>{data.trainer.username} / Edits | Karta Yaxu</title>
</svelte:head>

<div class="edits-page">
  <div class="page-header">
    <h2>{data.trainer.username.toUpperCase()} / EDITS</h2>
    <a href="/trainer/{data.trainer.username}" class="back-link">← BACK TO PROFILE</a>
  </div>

  {#if data.edits.length === 0}
    <div class="empty-state">
      <p>No edits yet.</p>
      <p class="hint">Changes made in the Editor will be recorded here.</p>
    </div>
  {:else}
    <div class="edits-table">
      <div class="table-header">
        <span class="col-id">#</span>
        <span class="col-table">TABLE</span>
        <span class="col-record">RECORD ID</span>
        <span class="col-date">DATE</span>
      </div>
      {#each data.edits as edit (edit.id)}
        <div class="table-row">
          <span class="col-id row-id">{edit.id}</span>
          <span class="col-table row-table">{edit.tableName}</span>
          <span class="col-record row-record">{edit.recordId}</span>
          <span class="col-date row-date">{formatDate(edit.editedAt)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .edits-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .back-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    text-decoration: none;
    width: fit-content;
  }

  .back-link:hover {
    text-decoration: underline;
  }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-8);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint {
    font-size: 12px;
    color: var(--theme-text-muted);
  }

  .edits-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow: hidden;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 60px 1fr 100px 160px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-4);
    align-items: center;
  }

  .table-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .table-row {
    border-bottom: 1px solid var(--theme-border-primary);
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background-color: var(--theme-bg-hover);
  }

  .row-id {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .row-table {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-primary);
  }

  .row-record {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    text-align: right;
  }

  .row-date {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
    text-align: right;
  }
</style>
