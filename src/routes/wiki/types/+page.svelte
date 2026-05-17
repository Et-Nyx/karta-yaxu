<script lang="ts">
  import TypeIcon from '$lib/components/TypeIcon.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data } = $props();

  let searchQuery = $state('');

  let filteredTypes = $derived(
    searchQuery.trim()
      ? data.types.filter(t => t.name.toLowerCase().includes(searchQuery.toLowerCase()))
      : data.types
  );
</script>

<svelte:head>
  <title>Types | Karta Yaxu</title>
</svelte:head>

<div class="types-page">
  <div class="page-header">
    <h2>TYPES</h2>
    <SearchBar
      bind:value={searchQuery}
      placeholder="Search by name... (/)"
      count={filteredTypes.length}
      total={data.types.length}
    />
  </div>

  <div class="types-grid">
    {#each filteredTypes as t (t.id)}
      <a href="/wiki/types/{t.slug}" class="type-card card-interactive">
        <div class="type-content">
          <TypeIcon typeName={t.name} size="lg" />
          <span class="type-name">{t.name.toUpperCase()}</span>
        </div>
        <div class="type-bar" style="background-color: {t.hexColor ?? `var(--color-type-${t.name})`}"></div>
      </a>
    {/each}
  </div>
</div>

<style>
  .types-page {
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

  .types-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .type-card {
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
    text-align: center;
    min-height: 120px;
  }

  .type-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-2);
    padding: var(--spacing-4) var(--spacing-4) var(--spacing-3);
    flex: 1;
  }

  .type-name {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    letter-spacing: 1px;
  }

  .type-bar {
    height: 4px;
    width: 100%;
    flex-shrink: 0;
  }

  /* Mobile: smaller cards */
  @media (max-width: 768px) {
    .types-grid {
      grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    }

    .type-content {
      padding: var(--spacing-3) var(--spacing-2) var(--spacing-2);
    }

    .type-name {
      font-size: 8px;
    }
  }
</style>
