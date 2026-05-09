<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import MoveCategoryIcon from '$lib/components/MoveCategoryIcon.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  
  let { data } = $props();
  
  const m = data.move;
  
  let searchQuery = $state('');

  let filteredLearners = $derived(
    data.learners.filter(p => 
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
      p.pokeapiId.toString() === searchQuery
    )
  );
</script>

<svelte:head>
  <title>{m.name.toUpperCase()} | Karta Yaxu</title>
</svelte:head>

<div class="move-detail-page">
  <!-- Cabecalho do Move -->
  <header class="detail-header">
    <div class="title-group">
      <h1>{m.name.toUpperCase()}</h1>
      <MoveCategoryIcon category={m.damageClass || 'physical'} size="lg" />
      {#if m.typeName}
        <TypeBadge name={m.typeName} size="lg" />
      {/if}
    </div>
    
    <div class="stats-row">
      <div class="stat">
        <span class="label">POWER</span>
        <span class="value">{m.power !== null ? m.power : '-'}</span>
      </div>
      <div class="stat">
        <span class="label">ACCURACY</span>
        <span class="value">{m.accuracy !== null ? m.accuracy + '%' : '-'}</span>
      </div>
      <div class="stat">
        <span class="label">PP</span>
        <span class="value">{m.pp !== null ? m.pp : '-'}</span>
      </div>
      <div class="stat">
        <span class="label">PRIORITY</span>
        <span class="value">{m.priority}</span>
      </div>
      <div class="stat">
        <span class="label">TARGET</span>
        <span class="value">{m.target ? m.target.toUpperCase().replace('-', ' ') : '-'}</span>
      </div>
    </div>
  </header>

  <!-- Lista de Pokémon que aprendem -->
  <section class="learners-section">
    <div class="section-header">
      <h2>LEARNED BY</h2>
      <div class="search-bar">
        <input 
          type="text" 
          bind:value={searchQuery} 
          placeholder="Filter Pokémon..." 
          class="search-input"
        />
        <div class="results-count">
          {filteredLearners.length} / {data.learners.length}
        </div>
      </div>
    </div>
    
    <div class="pokemon-grid">
      {#each filteredLearners as p (p.id)}
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
    
    {#if filteredLearners.length === 0}
      <div class="empty-state">
        <p>No Pokémon found.</p>
      </div>
    {/if}
  </section>
</div>

<style>
  .move-detail-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-6);
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    flex-wrap: wrap;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .stats-row {
    display: flex;
    gap: var(--spacing-8);
    flex-wrap: wrap;
    border-top: 2px dashed var(--theme-border-primary);
    padding-top: var(--spacing-4);
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .stat .label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .stat .value {
    font-family: var(--font-mono);
    font-size: 20px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  .learners-section {
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

  .empty-state {
    text-align: center;
    padding: var(--spacing-8);
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }
</style>
