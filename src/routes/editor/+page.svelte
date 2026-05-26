<script lang="ts">
  import ImportPanel from '$lib/components/ImportPanel.svelte';
  import ImportLog from '$lib/components/ImportLog.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import DataTable from '$lib/components/DataTable.svelte';

  let { data } = $props();

  function formatDate(ts: Date | null): string {
    if (!ts) return '—';
    return new Date(ts).toLocaleString('en-US', {
      day: '2-digit', month: '2-digit', year: 'numeric',
      hour: '2-digit', minute: '2-digit', hour12: false
    });
  }

  // Target standard PokeAPI maximums for seeding progress
  const EXPECTED_MAX: Record<string, number> = {
    languages: 13,
    regions: 10,
    generations: 9,
    versionGroups: 32,
    versions: 49,
    natures: 25,
    eggGroups: 15,
    moveLearnMethods: 10,
    encounterMethods: 15,

    types: 19,
    abilities: 307,
    moves: 919,
    items: 2110,
    pokemon: 1025,
    species: 1025,
    pokedexes: 32,
    locations: 869
  };

  // Filter entities based on read-only vs editable
  let regularEntities = $derived(data.entities.filter(e => e.ready));
  let metadataEntities = $derived(data.entities.filter(e => !e.ready));

  // Entities tabs
  const ENTITIES_TABS = [
    { key: 'editable', label: 'EDITABLE ENTITIES' },
    { key: 'metadata', label: 'CORE METADATA' }
  ];
  let activeEntitiesTab = $state('editable');

  // Right column tab control
  const RIGHT_TABS = [
    { key: 'logs', label: 'IMPORT LOGS' },
    { key: 'edits', label: 'RECENT EDITS' }
  ];
  let activeRightTab = $state('logs');
</script>

<svelte:head>
  <title>Database Manager & Importer | Karta Yaxu</title>
</svelte:head>

<div class="dashboard">
  <header class="page-header">
    <h2>DATABASE MANAGER & IMPORTER</h2>
    <p class="lead">
      View database seeding progress and manage records. 
      Manual edits are saved under <code>source='edited'</code> to protect them from re-imports, while new records use <code>source='custom'</code>.
    </p>
  </header>

  <!-- Entities Status Tabs Section -->
  <section class="entities-section">
    <h3 class="section-title">DATABASE ENTITIES STATUS</h3>
    <Tabs tabs={ENTITIES_TABS} bind:active={activeEntitiesTab}>
      {#if activeEntitiesTab === 'editable'}
        <div class="entity-grid">
          {#each regularEntities as entity}
            {@const max = EXPECTED_MAX[entity.key] || 100}
            {@const pct = Math.min(100, Math.round((entity.counts.total / max) * 100))}

            <a href={entity.href} class="entity-card clickable" title="{pct}% Seeded ({entity.counts.total} / {max})">
              <div class="entity-head">
                <span class="entity-label">{entity.label.toUpperCase()}</span>
                <span class="entity-total">{entity.counts.total} / {max}</span>
              </div>

              <!-- Segmented Retro Progress Bar showing actual Seeding percentage -->
              <div class="progress-track">
                <div class="progress-fill" style="width: {pct}%"></div>
              </div>

              <div class="entity-sources">
                <span class="src src-pokeapi">API {entity.counts.pokeapi}</span>
                {#if entity.counts.edited > 0}
                  <span class="src src-edited">EDT {entity.counts.edited}</span>
                {/if}
                {#if entity.counts.custom > 0}
                  <span class="src src-custom">CST {entity.counts.custom}</span>
                {/if}
              </div>

              <div class="card-action">EDIT ({pct}%) →</div>
            </a>
          {/each}
        </div>
      {:else}
        <div class="entity-grid">
          {#each metadataEntities as entity}
            {@const max = EXPECTED_MAX[entity.key] || 100}
            {@const pct = Math.min(100, Math.round((entity.counts.total / max) * 100))}

            <div class="entity-card static" title="{pct}% Seeded ({entity.counts.total} / {max})">
              <div class="entity-head">
                <span class="entity-label">{entity.label.toUpperCase()}</span>
                <span class="entity-total">{entity.counts.total} / {max}</span>
              </div>

              <!-- Segmented Retro Progress Bar showing actual Seeding percentage -->
              <div class="progress-track">
                <div class="progress-fill" style="width: {pct}%"></div>
              </div>

              <div class="entity-sources">
                <span class="src src-pokeapi">API {entity.counts.pokeapi}</span>
                {#if entity.counts.edited > 0}
                  <span class="src src-edited">EDT {entity.counts.edited}</span>
                {/if}
                {#if entity.counts.custom > 0}
                  <span class="src src-custom">CST {entity.counts.custom}</span>
                {/if}
              </div>

              <div class="card-action static-badge">METADATA ({pct}%)</div>
            </div>
          {/each}
        </div>
      {/if}
    </Tabs>
  </section>

  <!-- Split Panel for seeding and log activities -->
  <div class="dashboard-split">
    <!-- Left Column: Seeding Console -->
    <div class="seeding-console">
      <h3 class="section-title">SEEDING CONSOLE</h3>
      <ImportPanel regions={data.regions} versionGroups={data.versionGroups} />
    </div>

    <!-- Right Column: Live Logs & Edits History -->
    <div class="activity-console">
      <h3 class="section-title">ACTIVITY MONITOR</h3>

      <Tabs tabs={RIGHT_TABS} bind:active={activeRightTab}>
        {#if activeRightTab === 'logs'}
          <ImportLog logs={data.logs} />
        {:else if activeRightTab === 'edits'}
          <div class="edits-tab-content">
            {#if data.recentEdits.length === 0}
              <div class="empty-state">
                <p>No manual edits recorded yet.</p>
                <p class="hint">Changes made in the sub-editors will appear in this log.</p>
              </div>
            {:else}
              {#snippet editsHead()}
                <th>WHEN</th>
                <th>WHO</th>
                <th>TABLE</th>
                <th>RECORD</th>
              {/snippet}

              {#snippet editsBody()}
                {#each data.recentEdits as edit (edit.id)}
                  <tr>
                    <td class="cell-date">{formatDate(edit.editedAt)}</td>
                    <td class="cell-user">{edit.username ?? '—'}</td>
                    <td class="cell-table">{edit.tableName.toUpperCase()}</td>
                    <td class="cell-record">#{edit.recordId}</td>
                  </tr>
                {/each}
              {/snippet}

              <DataTable head={editsHead} children={editsBody} compact={true} />
            {/if}
          </div>
        {/if}
      </Tabs>
    </div>
  </div>
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
    font-family: var(--font-display);
    font-size: 20px;
  }

  .lead {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
    line-height: 1.5;
  }

  .lead code {
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
    padding: 1px 4px;
    font-size: 10px;
    color: var(--theme-text-primary);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-4) 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
    letter-spacing: 0.5px;
  }

  /* Entity Grid & Cards */
  .entity-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: var(--spacing-4);
    padding: var(--spacing-2) 0;
  }

  .entity-card {
    display: flex;
    flex-direction: column;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    text-decoration: none;
    color: var(--theme-text-primary);
    box-shadow: 3px 3px 0 var(--theme-border-primary);
    transition: all 0.1s ease;
    box-sizing: border-box;
    height: 135px;
  }

  .entity-card.clickable:hover {
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 4px 4px 0 var(--theme-text-accent);
  }

  .entity-card.static {
    opacity: 0.85;
  }

  .entity-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: var(--spacing-3);
  }

  .entity-label {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .entity-total {
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: bold;
    color: var(--theme-text-primary);
  }

  .entity-card.clickable:hover .entity-total {
    color: var(--theme-text-accent);
  }

  /* Retro Segmented Progress Bar */
  .progress-track {
    width: 100%;
    height: 8px;
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    margin-bottom: var(--spacing-3);
    box-sizing: border-box;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: repeating-linear-gradient(
      90deg,
      var(--theme-text-accent) 0px,
      var(--theme-text-accent) 6px,
      transparent 6px,
      transparent 8px
    );
  }

  .entity-sources {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-2);
    margin-bottom: auto;
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
    color: var(--color-text-warning, #d4a017);
  }

  .card-action {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    margin-top: var(--spacing-2);
    text-align: right;
    font-weight: bold;
  }

  .static-badge {
    color: var(--theme-text-muted);
  }

  /* Stacked Console & Monitor Layout */
  .dashboard-split {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .edits-tab-content {
    background-color: var(--theme-bg-primary);
    padding: var(--spacing-4);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    height: 480px;
    box-sizing: border-box;
    overflow-y: auto;
  }

  .cell-date { color: var(--theme-text-muted); }
  .cell-user { font-weight: bold; }
  .cell-table { color: var(--theme-text-accent); }
  .cell-record { color: var(--theme-text-secondary); text-align: right; }

  .empty-state {
    font-family: var(--font-mono);
    color: var(--theme-text-muted);
    padding: var(--spacing-6);
    text-align: center;
  }

  .empty-state p {
    font-size: 12px;
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint {
    font-size: 10px;
  }
</style>
