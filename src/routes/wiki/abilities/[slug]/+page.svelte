<script lang="ts">
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  
  let { data } = $props();
  
  const a = data.ability;
  
  let searchQuery = $state('');

  let filteredPokemon = $derived(
    data.pokemonList.filter(p => 
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
      p.pokeapiId.toString() === searchQuery
    )
  );
</script>

<svelte:head>
  <title>{a.name.toUpperCase().replace('-', ' ')} | Karta Yaxu</title>
</svelte:head>

<div class="ability-detail-page">
  <header class="detail-header">
    <h1>{a.name.toUpperCase().replace('-', ' ')}</h1>
  </header>

  <section class="pokemon-section">
    <div class="section-header">
      <h2>POKÉMON WITH THIS ABILITY</h2>
      <div class="search-bar">
        <input 
          type="text" 
          bind:value={searchQuery} 
          placeholder="Filter Pokémon..." 
          class="search-input"
        />
        <div class="results-count">
          {filteredPokemon.length} / {data.pokemonList.length}
        </div>
      </div>
    </div>
    
    <div class="pokemon-grid">
      {#each filteredPokemon as p (p.id)}
        <div class="card-wrapper" class:hidden-ability={p.isHidden}>
          {#if p.isHidden}
            <div class="hidden-badge">HIDDEN</div>
          {/if}
          <PokemonCard 
            pokemon={{
              pokeapiId: p.pokeapiId || 0,
              name: p.name,
              slug: p.slug,
              spriteUrl: p.spriteDefault,
              types: p.types
            }} 
          />
        </div>
      {/each}
    </div>
    
    {#if filteredPokemon.length === 0}
      <div class="empty-state">
        <p>No Pokémon found.</p>
      </div>
    {/if}
  </section>
</div>

<style>
  .ability-detail-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-6);
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .pokemon-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-4);
  }

  .section-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
    font-size: 18px;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
  }

  .search-input {
    width: 200px;
    padding: var(--spacing-2) var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    font-family: var(--font-mono);
    font-size: 12px;
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

  .card-wrapper {
    position: relative;
  }

  .hidden-badge {
    position: absolute;
    top: -10px;
    right: -10px;
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    font-family: var(--font-display);
    font-size: 8px;
    padding: 4px 8px;
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    z-index: 2;
    transform: rotate(5deg);
  }

  .empty-state {
    text-align: center;
    padding: var(--spacing-8);
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }
</style>
