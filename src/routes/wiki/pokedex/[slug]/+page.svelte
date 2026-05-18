<script lang="ts">
  import DataTable from '$lib/components/DataTable.svelte';
  import TypeBadge from '$lib/components/TypeBadge.svelte';

  let { data } = $props();

  function formatName(name: string): string {
    return name.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  }
</script>

<svelte:head>
  <title>{data.pokedex.name} | Pokédex | Karta Yaxu</title>
</svelte:head>

<div class="pokedex-detail">
  <div class="detail-header">
    <h2>{formatName(data.pokedex.name).toUpperCase()}</h2>
    <div class="header-meta">
      {#if data.pokedex.regionName}
        <span class="meta-badge">{data.pokedex.regionName.toUpperCase()}</span>
      {/if}
      {#if data.pokedex.isMainSeries}
        <span class="meta-badge accent">MAIN SERIES</span>
      {/if}
      <span class="meta-count">{data.entries.length} POKÉMON</span>
    </div>
  </div>

  {#if data.entries.length === 0}
    <div class="empty-state">
      <p>No Pokémon entries found for this pokédex.</p>
      <p class="hint">Import Pokémon data and game metadata to populate this pokédex.</p>
    </div>
  {:else}
    <DataTable compact>
      {#snippet head()}
        <th style="width: 48px; text-align: right">#</th>
        <th style="width: 56px"></th>
        <th>NAME</th>
        <th style="text-align: right">TYPE</th>
      {/snippet}
      {#each data.entries as entry (entry.entryNumber)}
        <tr>
          <td class="entry-num">{entry.entryNumber}</td>
          <td class="col-sprite">
            {#if entry.spriteUrl}
              <img src={entry.spriteUrl} alt={entry.name} class="sprite" loading="lazy" />
            {:else}
              <div class="sprite-placeholder"></div>
            {/if}
          </td>
          <td class="entry-name">
            <a href="/wiki/pokemon/{entry.slug}">{entry.name.toUpperCase()}</a>
          </td>
          <td class="col-types">
            <div class="types-stack">
              {#each entry.types as t}
                <TypeBadge name={t.name} hexColor={t.hexColor} size="sm" />
              {/each}
            </div>
          </td>
        </tr>
      {/each}
    </DataTable>
  {/if}
</div>

<style>
  .pokedex-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .detail-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .header-meta {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    flex-wrap: wrap;
  }

  .meta-badge {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    padding: 2px 8px;
  }

  .meta-badge.accent {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .meta-count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
  }

  .entry-num {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
    text-align: right;
  }

  .col-sprite {
    display: flex;
    justify-content: center;
  }

  .sprite {
    width: 56px;
    height: 56px;
    image-rendering: pixelated;
    object-fit: contain;
  }

  .sprite-placeholder {
    width: 56px;
    height: 56px;
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
  }

  .entry-name { font-family: var(--font-display); font-size: 10px; }
  .entry-name a { color: var(--theme-text-primary); text-decoration: none; }
  .entry-name a:hover { color: var(--theme-text-accent); }

  .col-types {
    text-align: right;
    vertical-align: middle;
  }

  .types-stack {
    display: inline-flex;
    flex-direction: column;
    gap: var(--spacing-1);
    align-items: flex-end;
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
</style>
