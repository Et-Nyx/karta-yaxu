<script lang="ts">
  import SearchBar from '$lib/components/SearchBar.svelte';
  import ItemCard from '$lib/components/ItemCard.svelte';

  let { data } = $props();

  let searchQuery = $state('');

  let filteredItems = $derived(
    searchQuery.trim()
      ? data.items.filter(i =>
          i.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (i.category && i.category.toLowerCase().includes(searchQuery.toLowerCase()))
        )
      : data.items
  );
</script>

<svelte:head>
  <title>Items | Karta Yaxu</title>
</svelte:head>

<div class="items-page">
  <div class="page-header">
    <h2>ITEMS</h2>
    <SearchBar
      bind:value={searchQuery}
      placeholder="Search by name or category... (/)"
      count={filteredItems.length}
      total={data.items.length}
    />
  </div>

  {#if filteredItems.length === 0}
    <p class="empty">No items found.</p>
  {:else}
    <div class="items-grid">
      {#each filteredItems as item (item.id)}
        <ItemCard
          name={item.name}
          slug={item.slug}
          spriteUrl={item.spriteUrl}
          category={item.category}
          cost={item.cost}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .items-page {
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

  .items-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .empty {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    padding: var(--spacing-5) 0;
    text-align: center;
  }
</style>
