<script lang="ts">
  import { onMount } from 'svelte';
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import { selectedPokedexSlug } from '$lib/stores/selectedPokedex';

  let { data, form } = $props();

  let query = $state('');
  let selectedPokedexId = $state<number | ''>('');
  let showCreate = $state(false);
  let prefillFetched = $state(false);
  let prefillMsg = $state('');

  onMount(() => {
    const storedSlug = $selectedPokedexSlug;
    if (storedSlug) {
      const matched = data.pokedexes.find((d: any) => d.slug === storedSlug);
      if (matched) selectedPokedexId = matched.id;
    }
  });

  $effect(() => {
    if (selectedPokedexId === '') {
      selectedPokedexSlug.set('');
    } else {
      const matched = data.pokedexes.find((d: any) => d.id === selectedPokedexId);
      if (matched) selectedPokedexSlug.set(matched.slug);
    }
  });

  let pokedexOptions = $derived([
    { value: '', label: '— Alphabetic —' },
    ...data.pokedexes.map((d: any) => ({
      value: d.id,
      label: d.name.toUpperCase()
    }))
  ]);

  let filtered = $derived.by(() => {
    const matchesSearch = (p: any) =>
      p.slug.includes(query.toLowerCase()) || p.name.toLowerCase().includes(query.toLowerCase()) ||
      p.pokeapiId?.toString() === query;

    if (selectedPokedexId === '') {
      return data.pokemon.filter(matchesSearch);
    }

    const activeIds = data.pokedexEntriesMap[selectedPokedexId] || [];
    const activeIdSet = new Set(activeIds);

    const list = data.pokemon.filter((p: any) => activeIdSet.has(p.id) && matchesSearch(p));

    const idToIndex = new Map(activeIds.map((id: number, index: number) => [id, index]));
    list.sort((a: any, b: any) => (idToIndex.get(a.id) ?? 0) - (idToIndex.get(b.id) ?? 0));

    return list.map((p: any) => ({
      ...p,
      dexNumber: (idToIndex.get(p.id) ?? 0) + 1
    }));
  });

  function getTypes(pokemonId: number) {
    return data.typeRows
      .filter(t => t.pokemonId === pokemonId)
      .map(t => ({ name: t.name, hexColor: t.hexColor }));
  }

  function toggleCreate() {
    showCreate = !showCreate;
    // Reset prefill state when reopening so a fresh species pick prefills again
    if (showCreate) {
      prefillFetched = false;
      prefillMsg = '';
    }
  }

  async function onSpeciesChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const speciesId = target.value;
    if (!speciesId || prefillFetched) return;

    try {
      const res = await fetch(`/api/editor/base-form/${speciesId}`);
      const body = await res.json();
      if (!body.found) {
        prefillMsg = 'No Base Form found for this species — fill manually.';
        return;
      }

      const formEl = target.form as HTMLFormElement;
      const isEmpty = (v: string) => v === '' || v === '0';
      for (const [key, val] of Object.entries(body.pokemon)) {
        const input = formEl.elements.namedItem(key) as HTMLInputElement | null;
        if (input && isEmpty(input.value) && val != null) {
          input.value = String(val);
        }
      }
      prefillFetched = true;
      prefillMsg = `Prefilled from Base Form (${body.types.length} types, ${body.abilities.length} abilities, learnset copied on save)`;
    } catch (err) {
      prefillMsg = 'Failed to fetch Base Form.';
    }
  }
</script>

<svelte:head>
  <title>Edit Pokémon | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <div class="page-header">
    <h2>POKÉMON</h2>
    <SearchBar bind:value={query} placeholder="Search pokémon... (/)" count={filtered.length} total={selectedPokedexId === '' ? data.total : (data.pokedexEntriesMap[selectedPokedexId]?.length ?? 0)} />
    <div class="pokedex-select-container">
      <label for="pokedexSelect">POKÉDEX:</label>
      <RetroSelect id="pokedexSelect" bind:value={selectedPokedexId} options={pokedexOptions} class="pokedex-select" />
    </div>
  </div>

  <div class="header-actions">
    <button type="button" class="new-btn" onclick={toggleCreate}>
      {showCreate ? '× CANCEL' : '+ NEW POKÉMON'}
    </button>
  </div>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE POKÉMON</h3>
      <form action="?/create" method="post" use:enhance class="create-form">
        <div class="field">
          <label for="c-name">NAME</label>
          <input id="c-name" name="name" type="text" maxlength="80" required value={form?.createValues?.name ?? ''}>
        </div>
        <div class="field">
          <label for="c-slug">SLUG</label>
          <input id="c-slug" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required value={form?.createValues?.slug ?? ''}>
        </div>
        <div class="field">
          <label for="c-species">SPECIES</label>
          <select id="c-species" name="speciesId" required onchange={onSpeciesChange}>
            <option value="">— pick a species —</option>
            {#each data.species as s}
              <option value={s.id} selected={String(form?.createValues?.speciesId ?? '') === String(s.id)}>
                #{String(s.dexNumber).padStart(4, '0')} {s.name}
              </option>
            {/each}
          </select>
          {#if prefillMsg}<span class="prefill-msg">{prefillMsg}</span>{/if}
        </div>
        <div class="field">
          <label for="c-height">HEIGHT</label>
          <input id="c-height" name="height" type="number" min="0" required value={form?.createValues?.height ?? 0}>
        </div>
        <div class="field">
          <label for="c-weight">WEIGHT</label>
          <input id="c-weight" name="weight" type="number" min="0" required value={form?.createValues?.weight ?? 0}>
        </div>
        <div class="stats-row">
          <div class="field stat-field">
            <label for="c-hp">HP</label>
            <input id="c-hp" name="hp" type="number" min="0" max="255" required value={form?.createValues?.hp ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-atk">ATK</label>
            <input id="c-atk" name="atk" type="number" min="0" max="255" required value={form?.createValues?.atk ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-def">DEF</label>
            <input id="c-def" name="def" type="number" min="0" max="255" required value={form?.createValues?.def ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-spa">SPA</label>
            <input id="c-spa" name="spa" type="number" min="0" max="255" required value={form?.createValues?.spa ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-spd">SPD</label>
            <input id="c-spd" name="spd" type="number" min="0" max="255" required value={form?.createValues?.spd ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-spe">SPE</label>
            <input id="c-spe" name="spe" type="number" min="0" max="255" required value={form?.createValues?.spe ?? 0}>
          </div>
        </div>
        <div class="form-actions">
          <button type="submit" class="save-btn">CREATE</button>
          {#if form?.createError}
            <span class="form-error">{form.createError}</span>
          {/if}
        </div>
      </form>
    </section>
  {/if}

  {#if filtered.length === 0}
    <div class="empty-state">
      <p>No pokémon yet.</p>
      <p class="hint">Click "+ NEW POKÉMON" above to create the first one.</p>
    </div>
  {:else}
    <div class="pokemon-grid">
      {#each filtered as p (p.id)}
        <div class="card-wrapper">
          <PokemonCard
            pokemon={{
              pokeapiId: p.pokeapiId || 0,
              name: p.name,
              slug: p.slug,
              spriteUrl: p.spriteDefault,
              types: getTypes(p.id),
              dexNumber: p.dexNumber,
            }}
          />
          <a href="/editor/pokemon/{p.slug}" class="card-link-overlay" aria-label="Edit {p.name}"></a>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .entity-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
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

  .header-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-4);
  }

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .new-btn {
    font-family: var(--font-display);
    font-size: 9px;
    padding: var(--spacing-2) var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    cursor: pointer;
  }

  .new-btn:hover {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .create-section {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-3) 0;
  }

  .create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .field input,
  .field select {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }

  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .stats-row {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: var(--spacing-2);
  }

  .stat-field input {
    text-align: center;
  }

  .form-actions {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .save-btn {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
  }

  .save-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
  }

  .prefill-msg {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-accent);
    margin-top: var(--spacing-1);
  }

  .pokemon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-4);
  }

  .card-wrapper {
    position: relative;
  }

  .card-link-overlay {
    position: absolute;
    inset: 0;
    z-index: 1;
  }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-5);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint {
    font-size: 11px;
    color: var(--theme-text-muted);
  }
</style>
