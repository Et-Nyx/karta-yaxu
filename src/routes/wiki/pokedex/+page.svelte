<script lang="ts">
  let { data } = $props();

  function formatName(name: string): string {
    return name.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  }
</script>

<svelte:head>
  <title>Pokédex | Karta Yaxu</title>
</svelte:head>

<div class="pokedex-list-page">
  <div class="page-header">
    <h2>POKÉDEX</h2>
    <p class="subtitle">Regional pokédexes — Pokémon ordered by regional dex number</p>
  </div>

  {#if data.total === 0}
    <div class="empty-state">
      <p>No pokédex data imported yet.</p>
      <p class="hint">Use the <a href="/import">Import Panel</a> to import game metadata (includes Pokédexes).</p>
    </div>
  {:else}
    <div class="pokedex-grid">
      {#each data.pokedexes as pdex (pdex.id)}
        <a href="/wiki/pokedex/{pdex.slug}" class="pokedex-card card-interactive">
          <div class="pdex-name">{formatName(pdex.name)}</div>
          {#if pdex.regionName}
            <div class="pdex-region">{pdex.regionName.toUpperCase()}</div>
          {/if}
          {#if pdex.isMainSeries}
            <div class="pdex-badge">MAIN SERIES</div>
          {/if}
        </a>
      {/each}
    </div>
  {/if}
</div>

<style>
  .pokedex-list-page {
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

  .subtitle {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0;
  }

  .pokedex-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--spacing-4);
  }

  .pokedex-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
    padding: var(--spacing-4);
  }

  .pdex-name {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .pdex-region {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .pdex-badge {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-accent);
    margin-top: var(--spacing-1);
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

  .empty-state a {
    color: var(--theme-text-accent);
    text-decoration: none;
  }
</style>
