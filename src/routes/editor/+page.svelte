<script lang="ts">
  let { data } = $props();

  function formatDate(ts: Date | null): string {
    if (!ts) return '—';
    return new Date(ts).toLocaleString('pt-BR', {
      day: '2-digit', month: '2-digit', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<svelte:head>
  <title>Editor | Karta Yaxu</title>
</svelte:head>

<div class="dashboard">
  <header class="page-header">
    <h2>DATABASE EDITOR</h2>
    <p class="lead">Edite registros importados ou crie novos. Edições marcam <code>source='edited'</code> (protegido de re-import). Novos registros recebem <code>source='custom'</code>.</p>
  </header>

  <section>
    <h3 class="section-title">ENTITIES</h3>
    <div class="entity-grid">
      {#each data.entities as entity}
        {#if entity.ready && entity.href}
          <a href={entity.href} class="entity-card card-interactive">
            <div class="entity-head">
              <span class="entity-label">{entity.label.toUpperCase()}</span>
              <span class="entity-total">{entity.counts.total}</span>
            </div>
            <div class="entity-sources">
              <span class="src src-pokeapi">{entity.counts.pokeapi} pokeapi</span>
              <span class="src src-edited">{entity.counts.edited} edited</span>
              <span class="src src-custom">{entity.counts.custom} custom</span>
            </div>
          </a>
        {:else}
          <div class="entity-card disabled">
            <div class="entity-head">
              <span class="entity-label">{entity.label.toUpperCase()}</span>
              <span class="entity-total">{entity.counts.total}</span>
            </div>
            <div class="entity-sources">
              <span class="coming-soon">COMING SOON</span>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  </section>

  <section>
    <h3 class="section-title">RECENT EDITS</h3>
    {#if data.recentEdits.length === 0}
      <div class="empty-state">
        <p>No edits recorded yet.</p>
        <p class="hint">Start editing entities to populate the audit log.</p>
      </div>
    {:else}
      <div class="edits-table">
        <div class="table-header">
          <span>WHEN</span>
          <span>WHO</span>
          <span>TABLE</span>
          <span>RECORD</span>
        </div>
        {#each data.recentEdits as edit (edit.id)}
          <div class="table-row">
            <span class="cell-date">{formatDate(edit.editedAt)}</span>
            <span class="cell-user">{edit.username ?? '—'}</span>
            <span class="cell-table">{edit.tableName}</span>
            <span class="cell-record">#{edit.recordId}</span>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .dashboard {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .lead {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0;
  }

  .lead code {
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
    padding: 1px 4px;
    font-size: 11px;
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-3) 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .entity-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
  }

  .entity-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    text-decoration: none;
    color: var(--theme-text-primary);
  }

  .entity-card.disabled {
    opacity: 0.5;
  }

  .entity-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .entity-label {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .entity-total {
    font-family: var(--font-display);
    font-size: 18px;
    color: var(--theme-text-accent);
  }

  .entity-sources {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }

  .src {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .src-edited {
    color: var(--theme-text-accent);
  }

  .src-custom {
    color: var(--color-text-warning);
  }

  .coming-soon {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .edits-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 140px 120px 1fr 80px;
    padding: var(--spacing-2) var(--spacing-3);
    gap: var(--spacing-3);
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

  .cell-date,
  .cell-user,
  .cell-table,
  .cell-record {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
  }

  .cell-record {
    text-align: right;
    color: var(--theme-text-muted);
  }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-5);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-1) 0;
  }

  .empty-state .hint {
    font-size: 11px;
    color: var(--theme-text-muted);
  }
</style>
