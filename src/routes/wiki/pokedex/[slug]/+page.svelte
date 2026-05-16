<script lang="ts">
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
    <div class="entries-table">
      <div class="table-header">
        <span class="col-num">#</span>
        <span class="col-sprite"></span>
        <span class="col-name">NAME</span>
        <span class="col-types">TYPE</span>
      </div>
      {#each data.entries as entry (entry.entryNumber)}
        <a href="/wiki/pokemon/{entry.slug}" class="table-row card-interactive">
          <span class="col-num entry-num">{entry.entryNumber}</span>
          <span class="col-sprite">
            {#if entry.spriteUrl}
              <img src={entry.spriteUrl} alt={entry.name} class="sprite" loading="lazy" />
            {:else}
              <div class="sprite-placeholder"></div>
            {/if}
          </span>
          <span class="col-name entry-name">{entry.name.toUpperCase()}</span>
          <span class="col-types">
            {#each entry.types as type}
              <TypeBadge name={type} size="sm" />
            {/each}
          </span>
        </a>
      {/each}
    </div>
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

  .entries-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow: hidden;
  }

  .table-header {
    display: grid;
    grid-template-columns: 48px 48px 1fr auto;
    align-items: center;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .table-row {
    display: grid;
    grid-template-columns: 48px 48px 1fr auto;
    align-items: center;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-4);
    border-bottom: 1px solid var(--theme-border-primary);
    border-left: none;
    border-right: none;
    border-top: none;
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background-color: var(--theme-bg-hover);
  }

  .col-num {
    text-align: right;
  }

  .col-sprite {
    display: flex;
    justify-content: center;
  }

  .col-types {
    display: flex;
    gap: var(--spacing-1);
    justify-content: flex-end;
  }

  .entry-num {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .sprite {
    width: 40px;
    height: 40px;
    image-rendering: pixelated;
    object-fit: contain;
  }

  .sprite-placeholder {
    width: 40px;
    height: 40px;
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
  }

  .entry-name {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
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
