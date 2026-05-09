<script lang="ts">
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  
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
  <title>Pokédex | Karta Yaxu</title>
</svelte:head>

<div class="pokedex-page">
  <div class="page-header">
    <h2>POKÉDEX</h2>
    <div class="search-bar">
      <input 
        type="text" 
        bind:value={searchQuery} 
        placeholder="Search by name or ID... (/)" 
        class="search-input"
      />
      <div class="results-count">
        {filteredPokemon.length} RESULTS
      </div>
    </div>
  </div>

  <div class="pokemon-grid">
    {#each filteredPokemon as p (p.id)}
      <PokemonCard 
        pokemon={{
          pokeapiId: p.pokeapiId || 0,
          name: p.name,
          slug: p.slug,
          spriteUrl: p.spriteDefault,
          types: p.types
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
