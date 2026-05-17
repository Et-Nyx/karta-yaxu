<script lang="ts">
  import MoveCard from '$lib/components/MoveCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data } = $props();

  let searchQuery = $state('');

  let filteredMoves = $derived(
    data.moves.filter(m =>
      m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (m.typeName && m.typeName.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );
</script>

<svelte:head>
  <title>Moves | Karta Yaxu</title>
</svelte:head>

<div class="moves-page">
  <div class="page-header">
    <h2>MOVES</h2>
    <SearchBar
      bind:value={searchQuery}
      placeholder="Search by name or type... (/)"
      count={filteredMoves.length}
      total={data.moves.length}
    />
  </div>

  <div class="moves-list">
    {#each filteredMoves as m (m.id)}
      <MoveCard
        name={m.name}
        slug={m.slug}
        typeName={m.typeName ?? 'normal'}
        typeHexColor={m.typeHexColor}
        damageClass={m.damageClass ?? 'physical'}
        power={m.power}
        accuracy={m.accuracy}
        pp={m.pp}
      />
    {/each}
  </div>
</div>

<style>
  .moves-page {
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

  .moves-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }


</style>
