<script lang="ts">
  import { enhance } from '$app/forms';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import { pageSource } from '$lib/stores/pageSource';

  let { data, form } = $props();

  $effect(() => { pageSource.set(data.ability.source); });

  let saved = $state(false);
  let effectsSaved = $state(false);

  function handleSave() {
    saved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        saved = true;
        setTimeout(() => { saved = false; }, 2500);
      }
    };
  }

  function handleEffectsSave() {
    effectsSaved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        effectsSaved = true;
        setTimeout(() => { effectsSaved = false; }, 2500);
      }
    };
  }

  let ability = $derived(form?.values ?? data.ability);

  let isMainSeriesChecked = $derived(
    form?.values
      ? form.values.isMainSeries === 'true' || form.values.isMainSeries === 'on'
      : !!data.ability.isMainSeries
  );

  // Version selector
  let selectedVersionGroupId = $state<number | null>(null);

  $effect(() => {
    if (selectedVersionGroupId === null && data.flavorTexts.length > 0) {
      selectedVersionGroupId = data.flavorTexts[data.flavorTexts.length - 1].versionGroupId;
    }
  });

  let selectedFlavorText = $derived(
    data.flavorTexts.find((f: { versionGroupId: number }) => f.versionGroupId === selectedVersionGroupId)?.flavorText ?? null
  );

  // Pokemon search
  let searchQuery = $state('');

  let filteredPokemon = $derived(
    data.pokemonList.filter((p: { name: string; pokeapiId: number | null }) =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (p.pokeapiId?.toString() ?? '') === searchQuery
    )
  );
</script>

<svelte:head>
  <title>Edit {data.ability.name} | Karta Yaxu</title>
</svelte:head>

<div class="ability-detail-page">
  <!-- Header card — identical layout to wiki, with editable inputs + meta-strip -->
  <header class="detail-header">
    <form action="?/update" method="post" use:enhance={handleSave}>
      <div class="title-group">
        <input
          class="h1-input"
          name="name"
          type="text"
          maxlength="50"
          required
          value={ability.name ?? ''}
        />
      </div>

      <div class="meta-strip">
        <div class="meta-field">
          <label>SLUG</label>
          <input type="text" name="slug" maxlength="50" pattern="[a-z0-9-]+" required value={ability.slug ?? ''} />
        </div>
        <div class="meta-field">
          <label>GENERATION ID</label>
          <input type="number" name="generationId" min="1" value={ability.generationId ?? ''} />
        </div>
        <div class="meta-field meta-field-check">
          <label class="check-label">
            <input
              type="checkbox"
              name="isMainSeries"
              value="true"
              checked={isMainSeriesChecked}
            />
            MAIN SERIES
          </label>
        </div>
        <button type="submit" class="save-btn">SAVE</button>
        <a href="." class="cancel-link">CANCEL</a>
        {#if form?.updateError}
          <span class="form-error">{form.updateError}</span>
        {/if}
        {#if saved}
          <span class="save-ok">✓ SAVED</span>
        {/if}
      </div>
    </form>
  </header>

  <!-- Detail grid: flavor texts (left) + effects editor (right) -->
  {#if data.shortEffect !== null || data.flavorTexts.length > 0}
    <div class="detail-grid">
      {#if data.flavorTexts.length > 0}
        <div class="left-col">
          <VersionSelector
            bind:value={selectedVersionGroupId}
            options={data.flavorTexts.map((f: { versionGroupId: number; versionGroupName: string }) => ({
              value: f.versionGroupId,
              label: f.versionGroupName.toUpperCase().replace(/-/g, ' '),
            }))}
          />
          {#if selectedFlavorText}
            <FlavorTextBox text={selectedFlavorText} />
          {/if}
        </div>
      {/if}

      <div class="right-col">
        <InfoBox title="EFFECT">
          <form action="?/updateEffects" method="post" use:enhance={handleEffectsSave} class="effects-form">
            <div class="field">
              <label for="effect">EFFECT</label>
              <textarea id="effect" name="effect" rows="4" maxlength="4000">{data.effect ?? ''}</textarea>
            </div>
            <div class="field">
              <label for="short">SHORT EFFECT</label>
              <textarea id="short" name="shortEffect" rows="2" maxlength="2000">{data.shortEffect ?? ''}</textarea>
            </div>
            <div class="effects-actions">
              <button type="submit" class="save-btn">SAVE EFFECTS</button>
              {#if form?.effectsError}
                <span class="form-error">{form.effectsError}</span>
              {/if}
              {#if effectsSaved}
                <span class="save-ok">✓ SAVED</span>
              {/if}
            </div>
          </form>
        </InfoBox>
      </div>
    </div>
  {/if}

  <!-- Pokemon with this ability (read-only, identical to wiki) -->
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
              types: p.types,
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

  <!-- Resource + delete forms -->
  {#if data.ability.source === 'edited' && data.ability.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button
        type="submit"
        class="resource-btn"
        onclick={(e) => { if (!confirm('Re-import this ability from PokeAPI? All your edits will be lost.')) e.preventDefault(); }}
      >
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Reverts to canonical PokeAPI data; source flips back to 'pokeapi'.</span>
    </form>
  {/if}

  {#if data.ability.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS ABILITY</button>
      <span class="delete-hint">Apenas registros custom podem ser deletados.</span>
    </form>
  {/if}
</div>

<style>
  .ability-detail-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  /* ── Header card ── */

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-6);
    box-shadow: var(--shadow-card);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header form {
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

  /* h1-style input */
  .h1-input {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    width: 100%;
    max-width: 500px;
    outline: none;
    padding: 0;
    margin: 0;
  }

  .h1-input:focus {
    border-bottom-color: var(--theme-text-accent);
  }

  /* Meta strip */
  .meta-strip {
    border-top: 2px dashed var(--theme-border-primary);
    padding-top: var(--spacing-3);
    display: flex;
    gap: var(--spacing-4);
    align-items: center;
    flex-wrap: wrap;
  }

  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .meta-field input[type="text"],
  .meta-field input[type="number"],
  .meta-field select {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    padding: 2px var(--spacing-1);
  }

  .meta-field input[type="text"]:focus,
  .meta-field input[type="number"]:focus,
  .meta-field select:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .meta-field-check {
    justify-content: center;
  }

  .check-label {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    cursor: pointer;
  }

  .check-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--theme-text-accent);
  }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .wiki-link {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-accent);
    text-decoration: none;
  }
  .wiki-link:hover { text-decoration: underline; }

  .save-btn {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-1) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
  }
  .save-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .cancel-link {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-muted);
    text-decoration: none;
  }
  .cancel-link:hover { text-decoration: underline; }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
  }

  .save-ok {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-accent);
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

  /* Effects form inside InfoBox */
  .effects-form {
    display: flex;
    flex-direction: column;
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

  .field textarea {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    resize: vertical;
  }

  .field textarea:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .effects-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  /* ── Pokemon section ── */

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
    font-family: var(--font-display);
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

  /* ── Resource / delete forms ── */

  .resource-form {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .resource-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-secondary);
    cursor: pointer;
  }
  .resource-btn:hover {
    border-color: var(--theme-text-primary);
    color: var(--theme-text-primary);
  }

  .resource-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .delete-form {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .delete-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--color-text-error);
    color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-btn:hover {
    background-color: var(--color-text-error);
    color: var(--theme-bg-primary);
  }

  .delete-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  @media (max-width: 768px) {
    .h1-input { font-size: 18px; }
    .detail-header { padding: var(--spacing-4); gap: var(--spacing-4); }
    .detail-grid { grid-template-columns: minmax(0, 1fr); }
    .left-col, .right-col { min-width: 0; }
  }
</style>
