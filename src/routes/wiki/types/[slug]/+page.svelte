<script lang="ts">
  import TypeIcon from '$lib/components/TypeIcon.svelte';
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';

  let { data } = $props();

  const t = data.typeData;

  // Agrupar matchups
  const doubleDamageTo = data.attackingList.filter(x => x.multiplier === 2.0);
  const halfDamageTo = data.attackingList.filter(x => x.multiplier === 0.5);
  const noDamageTo = data.attackingList.filter(x => x.multiplier === 0.0);

  const doubleDamageFrom = data.defendingList.filter(x => x.multiplier === 2.0);
  const halfDamageFrom = data.defendingList.filter(x => x.multiplier === 0.5);
  const noDamageFrom = data.defendingList.filter(x => x.multiplier === 0.0);

  let searchQuery = $state('');

  let filteredPokemon = $derived(
    data.pokemonList.filter(p =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.pokeapiId.toString() === searchQuery
    )
  );
</script>

<svelte:head>
  <title>{t.name.toUpperCase()} Type | Karta Yaxu</title>
</svelte:head>

<div class="type-detail-page">
  <header class="detail-header">
    <div class="header-content">
      <TypeIcon typeName={t.name} size="lg" />
      <div class="header-title">
        <span class="type-label">{t.name.toUpperCase()}</span>
        <h1>TYPE</h1>
      </div>
    </div>
    <div class="header-bar" style="background-color: var(--color-type-{t.name})"></div>
  </header>

  <!-- Damage Matchups -->
  <section class="matchups-grid">
    <div class="matchup-box">
      <h3>OFFENSIVE (ATTACKING)</h3>
      
      <div class="matchup-row">
        <span class="label super">SUPER EFFECTIVE (2x):</span>
        <div class="badges">
          {#each doubleDamageTo as m} <TypeBadge name={m.defenderName} size="sm" /> {/each}
          {#if doubleDamageTo.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
      
      <div class="matchup-row">
        <span class="label not-very">NOT VERY EFFECTIVE (0.5x):</span>
        <div class="badges">
          {#each halfDamageTo as m} <TypeBadge name={m.defenderName} size="sm" /> {/each}
          {#if halfDamageTo.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>

      <div class="matchup-row">
        <span class="label immune">NO EFFECT (0x):</span>
        <div class="badges">
          {#each noDamageTo as m} <TypeBadge name={m.defenderName} size="sm" /> {/each}
          {#if noDamageTo.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
    </div>

    <div class="matchup-box">
      <h3>DEFENSIVE (DEFENDING)</h3>
      
      <div class="matchup-row">
        <span class="label super">WEAK TO (2x):</span>
        <div class="badges">
          {#each doubleDamageFrom as m} <TypeBadge name={m.attackerName} size="sm" /> {/each}
          {#if doubleDamageFrom.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
      
      <div class="matchup-row">
        <span class="label not-very">RESISTS (0.5x):</span>
        <div class="badges">
          {#each halfDamageFrom as m} <TypeBadge name={m.attackerName} size="sm" /> {/each}
          {#if halfDamageFrom.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>

      <div class="matchup-row">
        <span class="label immune">IMMUNE TO (0x):</span>
        <div class="badges">
          {#each noDamageFrom as m} <TypeBadge name={m.attackerName} size="sm" /> {/each}
          {#if noDamageFrom.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
    </div>
  </section>

  <!-- Lista de Pokémon -->
  <section class="pokemon-section">
    <div class="section-header">
      <h2>POKÉMON OF THIS TYPE</h2>
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
  </section>
</div>

<style>
  .type-detail-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
    padding: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    padding: var(--spacing-6);
  }

  .header-title {
    display: flex;
    align-items: baseline;
    gap: var(--spacing-3);
  }

  .type-label {
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-secondary);
    letter-spacing: 2px;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .header-bar {
    height: 4px;
    width: 100%;
    flex-shrink: 0;
  }

  /* Mobile adjustments */
  @media (max-width: 768px) {
    .header-content {
      padding: var(--spacing-4);
    }

    h1 {
      font-size: 18px;
    }

    .type-label {
      font-size: 12px;
    }
  }

  .matchups-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
  }

  @media (max-width: 768px) {
    .matchups-grid {
      grid-template-columns: 1fr;
    }
  }

  .matchup-box {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .matchup-box h3 {
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-2) 0;
    border-bottom: 2px dashed var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
  }

  .matchup-row {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .matchup-row .label {
    font-family: var(--font-display);
    font-size: 10px;
  }

  .label.super { color: #3FA129; }
  .label.not-very { color: #D13B3B; }
  .label.immune { color: var(--theme-text-secondary); }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }

  .none {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
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
</style>
