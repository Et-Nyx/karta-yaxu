<script lang="ts">
  import { enhance } from '$app/forms';
  import { beforeNavigate } from '$app/navigation';
  import { onMount } from 'svelte';
  import { pageSource } from '$lib/stores/pageSource';
  import DataTable from '$lib/components/DataTable.svelte';
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import RetroButton from '$lib/components/RetroButton.svelte';

  let { data, form } = $props<{ data: any, form: any }>();

  // Reactively track Pokédex source
  $effect(() => {
    pageSource.set(isDirty ? 'unsaved' : data.pokedex.source);
  });

  // Local state for Pokédex metadata and entries
  let nameVal = $state(data.pokedex.name);
  let regionIdVal = $state(data.pokedex.regionId);
  let entries = $state([...data.entries]);

  let slugVal = $derived(
    nameVal
      .toLowerCase()
      .trim()
      .replace(/[^\w\s-]/g, '')
      .replace(/[\s_-]+/g, '-')
      .replace(/^-+|-+$/g, '')
  );

  let selectedPokemonToAdd = $state('');
  let entryError = $state('');

  let saved = $state(false);
  let isSaving = $state(false);

  let lastPokedexId = $state(data.pokedex.id);
  // Sync state variables when data loads/reloads
  $effect(() => {
    if (data.pokedex.id !== lastPokedexId) {
      lastPokedexId = data.pokedex.id;
      saved = false;
    }
    nameVal = data.pokedex.name;
    regionIdVal = data.pokedex.regionId;
    entries = [...data.entries];
  });

  // Derived dirty status: true if metadata or list order changed
  let isDirty = $derived.by(() => {
    if (nameVal !== data.pokedex.name) return true;
    if (regionIdVal !== data.pokedex.regionId) return true;
    if (entries.length !== data.entries.length) return true;
    for (let i = 0; i < entries.length; i++) {
      if (entries[i].pokemonId !== data.entries[i].pokemonId) return true;
    }
    return false;
  });

  // Derived evolution chain groups to treat them as unified draggable units
  let groups = $derived.by(() => {
    const list: Array<{ chainId: number; entries: typeof entries }> = [];
    let currentGroup: { chainId: number; entries: typeof entries } | null = null;

    for (const entry of entries) {
      if (!currentGroup || currentGroup.chainId !== entry.chainId) {
        currentGroup = { chainId: entry.chainId, entries: [] };
        list.push(currentGroup);
      }
      currentGroup.entries.push(entry);
    }
    return list;
  });

  // HTML5 Drag and Drop states (now tracking group indices)
  let draggedGroupIndex = $state<number | null>(null);
  let dragHoverGroupIndex = $state<number | null>(null);

  function handleDragStart(groupIndex: number) {
    if (data.pokedex.source !== 'custom') return;
    draggedGroupIndex = groupIndex;
  }

  function handleDragOver(e: DragEvent, groupIndex: number) {
    if (data.pokedex.source !== 'custom') return;
    if (draggedGroupIndex === null) return;

    e.preventDefault();
    dragHoverGroupIndex = groupIndex;
  }

  function handleDragLeave() {
    dragHoverGroupIndex = null;
  }

  function handleDrop(targetGroupIndex: number) {
    if (data.pokedex.source !== 'custom') return;
    if (draggedGroupIndex === null || draggedGroupIndex === targetGroupIndex) {
      draggedGroupIndex = null;
      dragHoverGroupIndex = null;
      return;
    }

    const items = [...groups];
    const [movedGroup] = items.splice(draggedGroupIndex, 1);
    items.splice(targetGroupIndex, 0, movedGroup);

    // Flatten back into entries
    const flattened = items.flatMap(g => g.entries);

    // Update dex numbers locally
    flattened.forEach((item, idx) => {
      item.entryNumber = idx + 1;
    });

    entries = flattened;
    draggedGroupIndex = null;
    dragHoverGroupIndex = null;
  }

  // Client-side local mutations for adding Pokémon
  async function handleAddEntry(e: SubmitEvent) {
    e.preventDefault();
    entryError = '';
    if (!selectedPokemonToAdd) return;

    try {
      const res = await fetch(`/api/pokemon/chain/${selectedPokemonToAdd}`);
      if (!res.ok) {
        entryError = 'Failed to fetch evolution chain details';
        return;
      }
      const newEntries = await res.json();
      if (!Array.isArray(newEntries) || newEntries.length === 0) {
        entryError = 'No evolution chain entries returned';
        return;
      }

      const currentIds = new Set(entries.map(e => e.pokemonId));
      const toAdd = newEntries.filter(ne => !currentIds.has(ne.pokemonId));

      if (toAdd.length === 0) {
        entryError = 'This Pokémon chain is already in the Pokédex';
        return;
      }

      const merged = [...entries, ...toAdd];
      merged.forEach((item, idx) => {
        item.entryNumber = idx + 1;
      });

      entries = merged;
      selectedPokemonToAdd = '';
    } catch (err) {
      console.error(err);
      entryError = 'An unexpected error occurred';
    }
  }

  // Client-side local mutations for removing chain
  function handleRemoveEntry(pokemonId: number, chainId: number) {
    if (!confirm('Are you sure you want to remove this evolution chain from the Pokédex?')) {
      return;
    }

    const filtered = entries.filter(e => e.chainId !== chainId);
    filtered.forEach((item, idx) => {
      item.entryNumber = idx + 1;
    });

    entries = filtered;
  }

  function handleSave() {
    isSaving = true;
    saved = false;
    return async ({ result, update }: any) => {
      isSaving = false;
      if (result.type === 'success') {
        saved = true;
        await update({ reset: false });
        setTimeout(() => saved = false, 2500);
      } else {
        await update({ reset: false });
      }
    };
  }

  // Intercept navigation if page has unsaved modifications
  beforeNavigate(({ cancel }) => {
    if (isSaving) return;
    if (isDirty && !confirm('You have unsaved changes. Leave without saving?')) {
      cancel();
    }
  });

  // Warn on browser tab closure
  onMount(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (isDirty) {
        e.preventDefault();
        e.returnValue = '';
      }
    };
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  });

  // Options mapping for selectors
  let regionOptions = $derived([
    { value: '', label: '— NONE —' },
    ...data.regions.map((r: any) => ({ value: r.id, label: r.name.toUpperCase() }))
  ]);

  let pokemonOptions = $derived([
    { value: '', label: '— PICK A POKÉMON TO ADD —' },
    ...data.pokemonList.map((p: any) => ({ value: p.id, label: p.name.toUpperCase() }))
  ]);
</script>

<svelte:head>
  <title>
    {data.pokedex.source === 'custom' ? 'Edit' : 'View'} {data.pokedex.name} | Pokédex Editor | Karta Yaxu
  </title>
</svelte:head>

<div class="pokedex-detail-editor">
  {#if data.pokedex.source === 'custom'}
    <form action="?/update" method="post" use:enhance={handleSave}>
      <!-- Hidden input containing the serialized custom order of entries -->
      <input type="hidden" name="pokemonIds" value={entries.map(e => e.pokemonId).join(',')} />

      <header class="detail-header">
        <div class="dex-num">#{entries.length.toString().padStart(3, '0')}</div>
        <div class="header-main">
          <input class="h1-input" name="name" type="text" maxlength={80} required bind:value={nameVal} />
          <div class="header-sub-row">
            {#if data.pokedex.regionName}
              <span class="genus">{data.pokedex.regionName.toUpperCase()} REGION</span>
            {/if}
            <span class="genus">POKÉDEX</span>
          </div>
        </div>
        <div class="types">
          <span class="meta-badge custom">CUSTOM</span>
        </div>
      </header>

      <div class="meta-strip">
        <input type="hidden" name="slug" value={slugVal} />
        <input type="hidden" name="isMainSeries" value="false" />

        <div class="meta-field">
          <label class="meta-label">REGION</label>
          <RetroSelect id="region" name="regionId" bind:value={regionIdVal} options={regionOptions} variant="editor" />
        </div>
        <div class="meta-actions">
          <RetroButton type="submit" variant="primary" disabled={!isDirty}>SAVE</RetroButton>
          <RetroButton
            type="button"
            variant="text"
            disabled={!isDirty}
            onclick={() => {
              if (confirm('You have unsaved changes. Cancel and discard all edits?')) {
                window.location.reload();
              }
            }}
          >
            CANCEL
          </RetroButton>
          {#if saved}
            <span class="save-ok">✓ SAVED</span>
          {/if}
          {#if form?.updateError}
            <span class="form-error">{form.updateError}</span>
          {/if}
          {#if isDirty}
            <span class="unsaved-hint">* Unsaved changes</span>
          {/if}
        </div>
      </div>
    </form>
  {:else}
    <header class="detail-header">
      <div class="dex-num">#{entries.length.toString().padStart(3, '0')}</div>
      <div class="header-main">
        <h1>{data.pokedex.name.toUpperCase()}</h1>
        <div class="header-sub-row">
          {#if data.pokedex.regionName}
            <span class="genus">{data.pokedex.regionName.toUpperCase()} REGION</span>
          {/if}
          <span class="genus">POKÉDEX</span>
        </div>
      </div>
      <div class="types">
        {#if data.pokedex.isMainSeries}
          <span class="meta-badge accent">MAIN SERIES</span>
        {/if}
      </div>
    </header>

    <div class="meta-strip">
      <div class="meta-field">
        <span class="meta-label-static">STATIC READ-ONLY POKÉDEX FROM POKEAPI</span>
      </div>
      <div class="meta-actions">
        <form action="?/copy" method="post" use:enhance class="copy-form">
          <RetroButton type="submit" variant="primary">COPY TO CUSTOM POKÉDEX</RetroButton>
        </form>
      </div>
    </div>
  {/if}

  <!-- Entries sub-editor -->
  <section class="entries-section">
    {#if data.pokedex.source === 'custom'}
      <form onsubmit={handleAddEntry} class="add-entry-form">
        <div class="field">
          <label for="add-pokemon">POKÉMON</label>
          <RetroSelect id="add-pokemon" name="pokemonId" bind:value={selectedPokemonToAdd} options={pokemonOptions} variant="editor" />
        </div>
        <div class="add-actions">
          <RetroButton type="submit" variant="secondary">+ ADD POKÉMON</RetroButton>
          {#if entryError}
            <span class="form-error">{entryError}</span>
          {/if}
        </div>
        <div class="evolution-note">
          * Note: Adding a Pokémon will automatically add all other related species in its Evolution Chain (e.g. adding Pikachu also adds Pichu before it and Raichu after it) to maintain evolution order.
        </div>
      </form>
    {/if}

    {#if entries.length === 0}
      <div class="empty-state">
        <p>No Pokémon entries found for this pokédex.</p>
        {#if data.pokedex.source === 'custom'}
          <p class="hint">Pick a Pokémon using the form above to add it and its evolution chain.</p>
        {:else}
          <p class="hint">This static Pokédex has no species entries loaded.</p>
        {/if}
      </div>
    {:else}
      <DataTable compact>
        {#snippet head()}
          <th style="width: 48px; text-align: right">#</th>
          <th style="width: 56px"></th>
          <th>NAME</th>
          <th style="text-align: right">TYPE</th>
          {#if data.pokedex.source === 'custom'}
            <th style="width: 140px; text-align: center">ACTIONS</th>
          {/if}
        {/snippet}
        {#each groups as g, groupIndex (g.chainId)}
          {#each g.entries as e, entryIndex (e.pokemonId)}
            <tr
              class="draggable-row"
              class:custom-row={data.pokedex.source === 'custom'}
              draggable={data.pokedex.source === 'custom'}
              class:dragging={draggedGroupIndex === groupIndex}
              class:drag-over={dragHoverGroupIndex === groupIndex}
              class:group-first={entryIndex === 0 && g.entries.length > 1}
              class:group-middle={entryIndex > 0 && entryIndex < g.entries.length - 1}
              class:group-last={entryIndex === g.entries.length - 1 && g.entries.length > 1}
              ondragstart={() => handleDragStart(groupIndex)}
              ondragover={(ev) => handleDragOver(ev, groupIndex)}
              ondragleave={handleDragLeave}
              ondrop={() => handleDrop(groupIndex)}
            >
              <td class="entry-num">{e.entryNumber}</td>
              <td class="col-sprite">
                {#if e.pokemonCard.spriteUrl}
                  <img src={e.pokemonCard.spriteUrl} alt={e.pokemonCard.name} class="sprite" loading="lazy" />
                {:else}
                  <div class="sprite-placeholder"></div>
                {/if}
              </td>
              <td class="entry-name">
                {#if data.pokedex.source === 'custom'}
                  <span class="name-drag-text">{e.pokemonCard.name.toUpperCase()}</span>
                {:else}
                  <a href="/wiki/pokemon/{e.pokemonCard.slug}">{e.pokemonCard.name.toUpperCase()}</a>
                {/if}
              </td>
              <td class="col-types">
                <div class="types-stack">
                  {#each e.pokemonCard.types as t}
                    <TypeBadge name={t.name} hexColor={t.hexColor} size="sm" />
                  {/each}
                </div>
              </td>
              {#if data.pokedex.source === 'custom'}
                <td class="col-actions">
                  <div class="remove-card-form">
                    <RetroButton type="button" variant="secondary" class="remove-row-btn" title="Remove Pokémon and its entire evolution chain" onclick={() => handleRemoveEntry(e.pokemonId, e.chainId)}>
                      REMOVE
                    </RetroButton>
                  </div>
                </td>
              {/if}
            </tr>
          {/each}
        {/each}
      </DataTable>
    {/if}
  </section>

  {#if data.pokedex.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <RetroButton type="submit" variant="secondary" class="delete-btn" onclick={(e) => { if (!confirm('Are you absolutely sure you want to delete this custom Pokédex and all its entries? This cannot be undone.')) e.preventDefault(); }}>
        DELETE THIS POKÉDEX
      </RetroButton>
      <span class="delete-hint">Only custom Pokédexes can be deleted. All entries are removed along with it.</span>
    </form>
  {/if}
</div>

<style>
  .pokedex-detail-editor {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  /* Header - matches pokemon page exactly */
  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: var(--shadow-card);
    flex-wrap: wrap;
  }

  .dex-num {
    font-family: var(--font-display);
    font-size: 16px;
    color: var(--theme-text-secondary);
    flex-shrink: 0;
  }

  .header-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
    min-width: 0;
  }

  .detail-header h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .h1-input {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    width: 100%;
    outline: none;
    padding: 0;
    text-transform: uppercase;
  }

  .h1-input:focus {
    border-bottom-color: var(--theme-text-accent);
  }

  .header-sub-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    flex-wrap: wrap;
  }

  .genus {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
  }

  .types {
    display: flex;
    gap: var(--spacing-2);
  }

  /* Meta-strip - matches pokemon page exactly */
  .meta-strip {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-4);
    border-left: 4px solid var(--theme-border-primary);
    border-right: 4px solid var(--theme-border-primary);
    border-bottom: 4px solid var(--theme-border-primary);
    border-top: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-6);
    background-color: var(--theme-bg-tertiary);
    box-shadow: var(--shadow-card);
    margin-top: -4px;
    box-sizing: border-box;
  }

  .meta-field {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .meta-label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-inverse);
    opacity: 0.85;
    white-space: nowrap;
  }

  .meta-label-static {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--color-text-warning);
    white-space: nowrap;
  }

  .meta-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    margin-left: auto;
  }

  .save-ok {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-accent);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-error);
  }

  .unsaved-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-warning);
    white-space: nowrap;
  }

  .meta-badge {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    padding: 2px 8px;
  }

  .meta-badge.accent {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .meta-badge.custom {
    color: var(--color-text-warning);
    border-color: var(--color-text-warning);
  }

  /* Entries sub-editor */
  .entries-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .add-entry-form {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--spacing-3);
    align-items: end;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    box-shadow: var(--shadow-card);
  }

  .add-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .evolution-note {
    grid-column: 1 / -1;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
    line-height: 1.3;
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

  /* Draggable Row styles */
  .draggable-row.custom-row {
    cursor: grab;
  }

  .draggable-row.dragging {
    opacity: 0.45;
    background-color: var(--theme-bg-hover) !important;
  }

  .draggable-row.drag-over {
    background-color: var(--theme-bg-hover) !important;
  }

  /* Evolution Group visual brackets */
  .draggable-row.group-first {
    border-top: 3px solid var(--color-text-warning) !important;
    border-left: 6px solid var(--color-text-warning) !important;
  }

  .draggable-row.group-middle {
    border-left: 6px solid var(--color-text-warning) !important;
    border-bottom: 1px dashed var(--theme-border-primary) !important;
  }

  .draggable-row.group-last {
    border-left: 6px solid var(--color-text-warning) !important;
    border-bottom: 3px solid var(--color-text-warning) !important;
  }

  .entry-num {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
    text-align: right;
  }

  .col-sprite {
    display: flex;
    justify-content: center;
  }

  .sprite {
    width: 56px;
    height: 56px;
    image-rendering: pixelated;
    object-fit: contain;
  }

  .sprite-placeholder {
    width: 56px;
    height: 56px;
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
  }

  .entry-name {
    font-family: var(--font-display);
    font-size: 10px;
  }

  .entry-name a {
    color: var(--theme-text-primary);
    text-decoration: none;
  }

  .entry-name a:hover {
    color: var(--theme-text-accent);
  }

  .name-drag-text {
    color: var(--theme-text-primary);
  }

  .drag-hint {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--theme-text-muted);
    margin-left: var(--spacing-2);
    font-weight: normal;
  }

  .col-types {
    text-align: right;
    vertical-align: middle;
  }

  .types-stack {
    display: inline-flex;
    flex-direction: column;
    gap: var(--spacing-1);
    align-items: flex-end;
  }

  .col-actions {
    text-align: center;
    vertical-align: middle;
  }

  .remove-card-form {
    display: inline-flex;
    justify-content: center;
  }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-8);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint {
    font-size: 12px;
    color: var(--theme-text-muted);
  }

  .delete-form {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    border-top: 2px solid var(--theme-border-primary);
    padding-top: var(--spacing-4);
  }

  .delete-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }
</style>
