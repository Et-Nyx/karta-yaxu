<script lang="ts">
  import SearchBar from '$lib/components/SearchBar.svelte';

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
    <SearchBar
      bind:value={searchQuery}
      placeholder="Search by name... (/)"
      count={filteredAbilities.length}
      total={data.abilities.length}
    />
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

  .abilities-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .ability-card {
    padding: var(--spacing-4);
    min-height: 120px;
    display: flex;
    align-items: center;
  }

  @media (max-width: 768px) {
    .abilities-list { grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); }
  }

  .ability-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }
</style>
