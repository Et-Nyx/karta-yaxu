<script lang="ts">
  import CollapsibleSection from '$lib/components/CollapsibleSection.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data } = $props();

  let searchQuery = $state('');

  let total = $derived(data.generations.reduce((sum, g) => sum + g.versionGroups.length, 0));

  let filteredGenerations = $derived(
    data.generations.map(gen => ({
      ...gen,
      versionGroups: searchQuery.trim()
        ? gen.versionGroups.filter(vg =>
            vg.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            vg.versions.some(v => v.name.toLowerCase().includes(searchQuery.toLowerCase()))
          )
        : gen.versionGroups,
    })).filter(gen => gen.versionGroups.length > 0)
  );

  let totalVisible = $derived(
    filteredGenerations.reduce((sum, g) => sum + g.versionGroups.length, 0)
  );

  let isSearching = $derived(searchQuery.trim().length > 0);

  let genOpen = $state<Record<string, boolean>>(
    Object.fromEntries(data.generations.map((g, i) => [g.slug, i === 0]))
  );

  $effect(() => {
    if (isSearching) {
      for (const g of filteredGenerations) {
        genOpen[g.slug] = true;
      }
    }
  });

  function formatGenName(name: string): string {
    return name.replace('generation-', 'GEN ').toUpperCase();
  }
</script>

<svelte:head>
  <title>Games | Karta Yaxu</title>
</svelte:head>

<div class="games-page">
  <div class="page-header">
    <h2>GAMES</h2>
    {#if total > 0}
      <SearchBar
        bind:value={searchQuery}
        placeholder="Search games... (/)"
        count={totalVisible}
        total={total}
      />
    {/if}
  </div>

  {#if data.generations.length === 0}
    <div class="empty-state">
      <p>No game data imported yet.</p>
      <p class="hint">Use the <a href="/import">Import Panel</a> to import game metadata.</p>
    </div>
  {:else if filteredGenerations.length === 0}
    <p class="empty">No games found matching "{searchQuery}".</p>
  {:else}
    {#each filteredGenerations as gen (gen.slug)}
      <CollapsibleSection
        title={formatGenName(gen.name)}
        count={gen.versionGroups.length}
        bind:open={genOpen[gen.slug]}
      >
        <div class="vg-grid">
          {#each gen.versionGroups as vg (vg.id)}
            <a href="/wiki/games/{vg.slug}" class="vg-card card-interactive">
              <div class="vg-name">{vg.name.toUpperCase().replace(/-/g, ' ')}</div>
              {#if vg.versions.length > 0}
                <div class="version-list">
                  {#each vg.versions as v}
                    <span class="version-pill">{v.name.toUpperCase()}</span>
                  {/each}
                </div>
              {/if}
            </a>
          {/each}
        </div>
      </CollapsibleSection>
    {/each}
  {/if}
</div>

<style>
  .games-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .empty {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }

  .vg-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .vg-card {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--spacing-2);
    padding: var(--spacing-4);
    min-height: 120px;
  }

  .vg-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    line-height: 1.6;
  }

  .version-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-1);
  }

  .version-pill {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    padding: 1px 6px;
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

  .empty-state .hint { font-size: 12px; color: var(--theme-text-muted); }
  .empty-state a { color: var(--theme-text-accent); text-decoration: none; }

  @media (max-width: 768px) {
    .vg-grid { grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); }
  }
</style>
