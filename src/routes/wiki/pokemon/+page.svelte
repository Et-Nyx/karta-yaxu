<script lang="ts">
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data } = $props();

  let searchQuery = $state('');

  let filteredPokemon = $derived(
    data.pokemon.filter(p => 
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
      p.pokeapiId.toString() === searchQuery
    )
  );
</script>

<svelte:head>
  <title>Pokémon | Karta Yaxu</title>
</svelte:head>

<div class="pokedex-page">
  <div class="page-header">
    <h2>POKÉMON</h2>
    <SearchBar
      bind:value={searchQuery}
      placeholder="Search by name or ID... (/)"
      count={filteredPokemon.length}
      total={data.pokemon.length}
    />
  </div>

  <div class="pokemon-grid">
    {#each filteredPokemon as p (p.id)}
      <PokemonCard
        pokemon={{
          pokeapiId: p.pokeapiId || 0,
          name: p.name,
          slug: p.slug,
          spriteUrl: p.spriteDefault,
          types: p.types,
          dexNumber: p.dexNumber,
          isDefault: p.isDefault,
        }}
      />
    {/each}
  </div>
  
  {#if filteredPokemon.length === 0}
    <div class="empty-state">
      <p>No Pokémon found matching "{searchQuery}".</p>
      <p>Make sure you have imported data in the Admin panel.</p>
    </div>
  {/if}
</div>

<style>
  .pokedex-page {
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

  .pokemon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-4);
  }

  .empty-state {
    text-align: center;
    padding: var(--spacing-8);
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }
</style>
