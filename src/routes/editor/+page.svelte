<script lang="ts">
  import ImportPanel from '$lib/components/ImportPanel.svelte';
  import ImportLog from '$lib/components/ImportLog.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import DataTable from '$lib/components/DataTable.svelte';
  import RetroInput from '$lib/components/RetroInput.svelte';
  import RetroButton from '$lib/components/RetroButton.svelte';

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

  // Details Modal state
  interface AuditDetailItem {
    type: 'import' | 'edit';
    id: number;
    title: string;
    fields: { label: string; value: string }[];
    diffs?: { field: string; prev: any; curr: any }[];
    comment: string;
  }
  let selectedItem = $state<AuditDetailItem | null>(null);
  let detailComment = $state('');
  let isSavingComment = $state(false);

  function computeFieldDiff(prevStr: string | null, newStr: string | null) {
    if (!prevStr && !newStr) return [];
    try {
      const prev = prevStr ? JSON.parse(prevStr) : {};
      const curr = newStr ? JSON.parse(newStr) : {};
      const allKeys = Array.from(new Set([...Object.keys(prev), ...Object.keys(curr)]));
      
      const diffs: { field: string; prev: any; curr: any }[] = [];
      for (const key of allKeys) {
        if (['createdAt', 'updatedAt', 'editedAt', 'timestamp', 'trainerId', 'id'].includes(key)) continue;
        
        const pVal = prev[key];
        const cVal = curr[key];
        
        if (JSON.stringify(pVal) !== JSON.stringify(cVal)) {
          diffs.push({
            field: key,
            prev: pVal === undefined || pVal === null ? '—' : (typeof pVal === 'object' ? JSON.stringify(pVal) : pVal),
            curr: cVal === undefined || cVal === null ? '—' : (typeof cVal === 'object' ? JSON.stringify(cVal) : cVal),
          });
        }
      }
      return diffs;
    } catch {
      return [];
    }
  }

  function openImportDetail(log: any) {
    selectedItem = {
      type: 'import',
      id: log.id,
      title: `IMPORT LOG ENTRY #${log.id}`,
      fields: [
        { label: 'TIME', value: formatDate(log.timestamp) },
        { label: 'ENTITY', value: log.entityType.toUpperCase() },
        { label: 'API ID', value: `#${log.pokeapiId}` },
        { label: 'ACTION', value: log.action.toUpperCase() },
        { label: 'REASON', value: log.reason || '—' }
      ],
      comment: log.comment || ''
    };
    detailComment = log.comment || '';
  }

  function openEditDetail(edit: any) {
    const diffs = computeFieldDiff(edit.previousData, edit.newData);
    selectedItem = {
      type: 'edit',
      id: edit.id,
      title: `MANUAL EDIT ENTRY #${edit.id}`,
      fields: [
        { label: 'TIME', value: formatDate(edit.editedAt) },
        { label: 'WHO', value: edit.username || '—' },
        { label: 'TABLE', value: edit.tableName.toUpperCase() },
        { label: 'RECORD ID', value: `#${edit.recordId}` }
      ],
      diffs,
      comment: edit.comment || ''
    };
    detailComment = edit.comment || '';
  }

  function closeDetailModal() {
    selectedItem = null;
  }

  async function saveAuditComment() {
    if (!selectedItem) return;
    isSavingComment = true;
    try {
      const res = await fetch('/api/editor/audit', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: selectedItem.type,
          id: selectedItem.id,
          comment: detailComment
        })
      });
      if (res.ok) {
        if (selectedItem.type === 'import') {
          const idx = data.logs.findIndex((l: any) => l.id === selectedItem!.id);
          if (idx !== -1) data.logs[idx].comment = detailComment;
        } else {
          const idx = data.recentEdits.findIndex((e: any) => e.id === selectedItem!.id);
          if (idx !== -1) data.recentEdits[idx].comment = detailComment;
        }
        closeDetailModal();
      } else {
        alert('Failed to save comment');
      }
    } catch (e) {
      console.error(e);
      alert('Error connecting to server');
    } finally {
      isSavingComment = false;
    }
  }
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
      <ImportPanel regions={data.regions} versionGroups={data.versionGroups} pokemonTotal={data.entities.find(e => e.key === 'pokemon')?.counts.total ?? 0} />
    </div>

    <!-- Right Column: Live Logs & Edits History -->
    <div class="activity-console">
      <h3 class="section-title">ACTIVITY MONITOR</h3>

      <Tabs tabs={RIGHT_TABS} bind:active={activeRightTab}>
        {#if activeRightTab === 'logs'}
          <ImportLog logs={data.logs} onRowClick={openImportDetail} />
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
                  <tr class="clickable-row" onclick={() => openEditDetail(edit)}>
                    <td class="cell-date">{formatDate(edit.editedAt)}</td>
                    <td class="cell-user">{edit.username ?? '—'}</td>
                    <td class="cell-table">{edit.tableName.toUpperCase()}</td>
                    <td class="cell-record">
                      #{edit.recordId}
                      {#if edit.comment}
                        <span class="comment-icon" title={edit.comment}>💬</span>
                      {/if}
                    </td>
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

{#if selectedItem}
  <div class="retro-modal-overlay">
    <div class="retro-modal">
      <div class="retro-modal-header">
        <h3>{selectedItem.title}</h3>
        <button type="button" class="btn-close" onclick={closeDetailModal}>×</button>
      </div>
      <div class="retro-modal-body">
        <div class="details-grid">
          {#each selectedItem.fields as f}
            <div class="detail-row">
              <span class="detail-label">{f.label}:</span>
              <span class="detail-value">{f.value}</span>
            </div>
          {/each}
        </div>

        {#if selectedItem.diffs && selectedItem.diffs.length > 0}
          <div class="diff-container">
            <h4>MODIFIED FIELDS:</h4>
            <table class="retro-diff-table">
              <thead>
                <tr>
                  <th>FIELD</th>
                  <th>BEFORE</th>
                  <th>AFTER</th>
                </tr>
              </thead>
              <tbody>
                {#each selectedItem.diffs as diff}
                  <tr>
                    <td class="diff-field">{diff.field.toUpperCase()}</td>
                    <td class="diff-prev">{diff.prev}</td>
                    <td class="diff-curr">{diff.curr}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {:else if selectedItem.type === 'edit'}
          <div class="no-diff-msg">No property changes recorded or identical values saved.</div>
        {/if}

        <div class="comment-section">
          <h4>PERSISTENT COMMENT / NOTE:</h4>
          <RetroInput
            type="text"
            placeholder="e.g. Verified and approved changes"
            bind:value={detailComment}
            class="modal-retro-input"
          />
        </div>

        <div class="modal-actions">
          <RetroButton onclick={saveAuditComment} disabled={isSavingComment}>
            {#if isSavingComment}SAVING...{:else}SAVE NOTE{/if}
          </RetroButton>
          <RetroButton onclick={closeDetailModal} variant="secondary">CLOSE</RetroButton>
        </div>
      </div>
    </div>
  </div>
{/if}

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

  .clickable-row {
    cursor: pointer;
    transition: background-color 0.1s ease;
  }

  .clickable-row:hover {
    background-color: rgba(0, 255, 0, 0.05) !important;
  }

  .comment-icon {
    font-size: 10px;
    margin-left: var(--spacing-2);
    vertical-align: middle;
  }

  /* Retro Audit Detail Modal styling */
  .retro-modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(0, 0, 0, 0.7);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 10000;
  }

  .retro-modal {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: 8px 8px 0 var(--theme-border-primary);
    width: 460px;
    max-width: 90%;
    display: flex;
    flex-direction: column;
    max-height: 85vh;
    overflow: hidden;
  }

  .retro-modal-header {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 4px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
  }

  .retro-modal-header h3 {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-inverse);
    margin: 0;
  }

  .retro-modal-header .btn-close {
    background: none;
    border: none;
    color: var(--theme-text-inverse);
    font-family: var(--font-mono);
    font-size: 16px;
    font-weight: bold;
    cursor: pointer;
    padding: 0 var(--spacing-1);
  }

  .retro-modal-body {
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
    overflow-y: auto;
  }

  .details-grid {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
    border-bottom: 2px dashed var(--theme-border-primary);
    padding-bottom: var(--spacing-3);
  }

  .detail-row {
    display: flex;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .detail-label {
    width: 100px;
    font-weight: bold;
    color: var(--theme-text-secondary);
  }

  .detail-value {
    color: var(--theme-text-primary);
  }

  .diff-container h4, .comment-section h4 {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .retro-diff-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 10px;
    margin-bottom: var(--spacing-2);
  }

  .retro-diff-table th {
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    font-family: var(--font-display);
    font-size: 8px;
    text-align: left;
    padding: var(--spacing-1) var(--spacing-2);
    border: 2px solid var(--theme-border-primary);
  }

  .retro-diff-table td {
    padding: var(--spacing-1) var(--spacing-2);
    border: 2px solid var(--theme-border-primary);
  }

  .diff-field {
    font-weight: bold;
  }

  .diff-prev {
    color: var(--theme-text-muted);
  }

  .diff-curr {
    color: var(--theme-text-accent);
    font-weight: bold;
  }

  .no-diff-msg {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
    border-bottom: 2px dashed var(--theme-border-primary);
    padding-bottom: var(--spacing-3);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-3);
    border-top: 2px dashed var(--theme-border-primary);
    padding-top: var(--spacing-3);
    margin-top: var(--spacing-2);
  }

  :global(.modal-retro-input) {
    width: 100%;
    box-sizing: border-box;
  }
</style>
