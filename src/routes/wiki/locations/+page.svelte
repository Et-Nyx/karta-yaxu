<script lang="ts">
  import SearchBar from '$lib/components/SearchBar.svelte';
  import LocationCard from '$lib/components/LocationCard.svelte';

  let { data } = $props();

  let searchQuery = $state('');

  let filteredRegions = $derived(
    data.regions.map(region => ({
      ...region,
      locations: searchQuery.trim()
        ? region.locations.filter(loc =>
            loc.name.toLowerCase().includes(searchQuery.toLowerCase())
          )
        : region.locations,
    })).filter(region => region.locations.length > 0)
  );

  let totalVisible = $derived(
    filteredRegions.reduce((sum, r) => sum + r.locations.length, 0)
  );
</script>

<svelte:head>
  <title>Locations | Karta Yaxu</title>
</svelte:head>

<div class="locations-page">
  <div class="page-header">
    <h2>LOCATIONS</h2>
    {#if data.total > 0}
      <SearchBar
        bind:value={searchQuery}
        placeholder="Search locations... (/)"
        count={totalVisible}
        total={data.total}
      />
    {/if}
  </div>

  {#if data.total === 0}
    <div class="empty-state">
      <p>No locations imported yet.</p>
      <p class="hint">Use the <a href="/import">Import Panel</a> to import location data from PokeAPI.</p>
    </div>
  {:else if filteredRegions.length === 0}
    <p class="empty">No locations found matching "{searchQuery}".</p>
  {:else}
    {#each filteredRegions as region}
      <section class="region-section">
        <h3 class="region-title">{region.name.toUpperCase()}</h3>
        <div class="location-grid">
          {#each region.locations as loc (loc.id)}
            <LocationCard name={loc.name} slug={loc.slug} />
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</div>

<style>
  .locations-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
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

  .empty {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }

  .region-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .region-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .location-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }
</style>
