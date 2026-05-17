<script lang="ts">
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import EffectText from '$lib/components/EffectText.svelte';

  let { data } = $props();

  const a = data.ability;

  let searchQuery = $state('');

  let filteredPokemon = $derived(
    data.pokemonList.filter(p =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.pokeapiId.toString() === searchQuery
    )
  );

  let selectedVersionGroupId = $state<number | null>(null);

  $effect(() => {
    if (selectedVersionGroupId === null && data.flavorTexts.length > 0) {
      selectedVersionGroupId = data.flavorTexts[data.flavorTexts.length - 1].versionGroupId;
    }
  });

  let selectedFlavorText = $derived(
    data.flavorTexts.find(f => f.versionGroupId === selectedVersionGroupId)?.flavorText ?? null
  );
</script>

<svelte:head>
  <title>{a.name.toUpperCase().replace('-', ' ')} | Karta Yaxu</title>
</svelte:head>

<div class="ability-detail-page">
  <header class="detail-header">
    <h1>{a.name.toUpperCase().replace('-', ' ')}</h1>
  </header>

  {#if data.shortEffect || data.flavorTexts.length > 0}
    <div class="detail-grid">
      {#if data.flavorTexts.length > 0}
        <div class="left-col">
          <VersionSelector
            bind:value={selectedVersionGroupId}
            options={data.flavorTexts.map(f => ({ value: f.versionGroupId, label: f.versionGroupName.toUpperCase().replace(/-/g, ' ') }))}
          />
          {#if selectedFlavorText}
            <FlavorTextBox text={selectedFlavorText} />
          {/if}
        </div>
      {/if}

      {#if data.shortEffect}
        <div class="right-col">
          <InfoBox title="EFFECT">
            <EffectText shortEffect={data.shortEffect} effect={data.effect} />
          </InfoBox>
        </div>
      {/if}
    </div>
  {/if}

  <section class="pokemon-section">
    <div class="section-header">
      <h2>POKÉMON WITH THIS ABILITY</h2>
      <SearchBar bind:value={searchQuery} placeholder="Filter Pokémon..." count={filteredPokemon.length} total={data.pokemonList.length} />
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
    box-shadow: var(--shadow-card);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  /* ── Detail grid ── */

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
    align-items: start;
  }

  .left-col, .right-col {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  @media (max-width: 768px) {
    h1 { font-size: 18px; }
    .detail-header { padding: var(--spacing-4); }
    .detail-grid { grid-template-columns: minmax(0, 1fr); }
    .left-col, .right-col { min-width: 0; }
  }

  /* ── Pokémon list ── */

  .pokemon-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
    font-size: 18px;
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
