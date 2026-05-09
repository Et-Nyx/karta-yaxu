<script lang="ts">
  let { data } = $props();

  let searchQuery = $state('');

  let filteredAbilities = $derived(
    data.abilities.filter(a => 
      a.name.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
</script>

<svelte:head>
  <title>Abilities | Karta Yaxu</title>
</svelte:head>

<div class="abilities-page">
  <div class="page-header">
    <h2>ABILITIES</h2>
    <div class="search-bar">
      <input 
        type="text" 
        bind:value={searchQuery} 
        placeholder="Search by name... (/)" 
        class="search-input"
      />
      <div class="results-count">
        {filteredAbilities.length} RESULTS
      </div>
    </div>
  </div>

  <div class="abilities-list">
    {#each filteredAbilities as a (a.id)}
      <a href="/wiki/abilities/{a.slug}" class="ability-card card-interactive">
        <div class="ability-name">{a.name.toUpperCase().replace('-', ' ')}</div>
      </a>
    {/each}
  </div>
</div>

<style>
  .abilities-page {
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

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
  }

  .search-input {
    flex: 1;
    max-width: 400px;
    padding: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .results-count {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .abilities-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: var(--spacing-4);
  }

  .ability-card {
    padding: var(--spacing-4);
  }

  .ability-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }
</style>
