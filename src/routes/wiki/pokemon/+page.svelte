<script lang="ts">
  import { onMount } from 'svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import { selectedPokedexSlug } from '$lib/stores/selectedPokedex';

  let { data } = $props();

  let searchQuery = $state('');
  let selectedPokedexId = $state<number | ''>('');

  // On mount: restore the stored Pokédex choice (match by slug)
  onMount(() => {
    const storedSlug = $selectedPokedexSlug;
    if (storedSlug) {
      const matched = data.pokedexes.find((d: any) => d.slug === storedSlug);
      if (matched) selectedPokedexId = matched.id;
    }
  });

  // When the user picks a different Pokédex, persist it to the store
  $effect(() => {
    if (selectedPokedexId === '') {
      selectedPokedexSlug.set('');
    } else {
      const matched = data.pokedexes.find((d: any) => d.id === selectedPokedexId);
      if (matched) selectedPokedexSlug.set(matched.slug);
    }
  });

  // 1. Build the retro pokedex options
  let pokedexOptions = $derived([
    { value: '', label: '— NATIONAL POKÉDEX (ALL) —' },
    ...data.pokedexes.map((d: any) => ({
      value: d.id,
      label: d.name.toUpperCase()
    }))
  ]);

  // 2. Derive the filtered and ordered Pokémon cards list
  let filteredPokemon = $derived.by(() => {
    const matchesSearch = (p: any) =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
      p.pokeapiId?.toString() === searchQuery;

    if (selectedPokedexId === '') {
      // Return National Pokédex order and default regional numbers
      return data.pokemon.filter(matchesSearch);
    }

    // Specific Pokédex is selected
    const activeIds = data.pokedexEntriesMap[selectedPokedexId] || [];
    const activeIdSet = new Set(activeIds);

    // Filter to only Pokémon in the selected Pokédex
    const list = data.pokemon.filter(p => activeIdSet.has(p.id) && matchesSearch(p));

    // Sort to match the Pokédex sequence order
    const idToIndex = new Map(activeIds.map((id, index) => [id, index]));
    list.sort((a, b) => (idToIndex.get(a.id) ?? 0) - (idToIndex.get(b.id) ?? 0));

    // Map each card to show its dynamic regional entry number in the Pokédex
    return list.map(p => {
      const idx = idToIndex.get(p.id) ?? 0;
      return {
        ...p,
        dexNumber: idx + 1 // Dynamic Pokédex regional number!
      };
    });
  });

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
      total={selectedPokedexId === '' ? data.pokemon.length : (data.pokedexEntriesMap[selectedPokedexId]?.length ?? 0)}
    />
    <div class="pokedex-select-container">
      <label for="pokedexSelect">POKÉDEX:</label>
      <RetroSelect id="pokedexSelect" bind:value={selectedPokedexId} options={pokedexOptions} class="pokedex-select" />
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
      {#if selectedPokedexId !== ''}
        <p>Make sure this Pokédex has entries, or try another select filter.</p>
      {:else}
        <p>Make sure you have imported data in the Admin panel.</p>
      {/if}
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

  .pokedex-select-container {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    width: 100%;
    box-sizing: border-box;
  }

  .pokedex-select-container label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    white-space: nowrap;
    flex-shrink: 0;
  }

  :global(.pokedex-select) {
    flex: 1;
    min-width: 0;
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
