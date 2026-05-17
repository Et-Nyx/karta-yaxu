<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.species.filter(s =>
      s.slug.includes(query.toLowerCase()) || s.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  function genName(id: number | null): string {
    if (id == null) return '—';
    return data.generations.find(g => g.id === id)?.name ?? `#${id}`;
  }
</script>

<svelte:head>
  <title>Edit Species | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>SPECIES</h2>
      <span class="count">{data.total} total · {filtered.length} shown</span>
    </div>
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW SPECIES'}
    </button>
  </header>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE SPECIES</h3>
      <p class="create-note">
        Creating a Species automatically generates its Base Form: a default <code>pokemon</code> row + default <code>pokemon_forms</code> row.
        You can edit the Base Form's stats/types/abilities afterward via <a href="/editor/pokemon">/editor/pokemon</a>.
      </p>
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
          <label for="c-dex">DEX #</label>
          <input id="c-dex" name="dexNumber" type="number" min="0" max="9999" required value={form?.createValues?.dexNumber ?? ''}>
        </div>
        <div class="field full">
          <label for="c-genus">GENUS</label>
          <input id="c-genus" name="genus" type="text" placeholder="Mouse Pokémon" value={form?.createValues?.genus ?? ''}>
        </div>
        <div class="field field-check">
          <label><input name="isBaby" type="checkbox" value="true"> BABY</label>
        </div>
        <div class="field field-check">
          <label><input name="isLegendary" type="checkbox" value="true"> LEGENDARY</label>
        </div>
        <div class="field field-check">
          <label><input name="isMythical" type="checkbox" value="true"> MYTHICAL</label>
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

  <SearchBar bind:value={query} placeholder="Search species..." count={filtered.length} total={data.total} />

  <div class="entity-table">
    <div class="table-header">
      <span>DEX</span>
      <span>SLUG</span>
      <span>NAME</span>
      <span>GENUS</span>
      <span>FLAGS</span>
      <span>GEN</span>
      <span>SOURCE</span>
      <span></span>
    </div>
    {#each filtered as s (s.id)}
      <div class="table-row">
        <span class="row-dex">#{s.dexNumber}</span>
        <span class="row-slug">{s.slug}</span>
        <span class="row-name">{s.name}</span>
        <span class="row-genus">{s.genus ?? '—'}</span>
        <span class="row-flags">
          {s.isBaby ? 'B ' : ''}{s.isLegendary ? 'L ' : ''}{s.isMythical ? 'M' : ''}
          {!s.isBaby && !s.isLegendary && !s.isMythical ? '—' : ''}
        </span>
        <span class="row-gen">{genName(s.generationId)}</span>
        <span class="src-badge src-{s.source}">{s.source}</span>
        <a href="/editor/species/{s.slug}" class="edit-link">EDIT →</a>
      </div>
    {/each}
  </div>
</div>

<style>
  .entity-page { display: flex; flex-direction: column; gap: var(--spacing-4); }
  .page-header { display: flex; justify-content: space-between; align-items: flex-end; }
  .page-header h2 { color: var(--theme-text-primary); margin: 0; }
  .count { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }

  .new-btn {
    font-family: var(--font-display); font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none; border: 2px solid var(--theme-border-primary); color: var(--theme-text-primary);
    cursor: pointer;
  }
  .new-btn:hover { border-color: var(--theme-text-accent); color: var(--theme-text-accent); }

  .create-section {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }
  .section-title { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-secondary); margin: 0 0 var(--spacing-3) 0; }

  .create-note {
    font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-3) 0;
  }
  .create-note code {
    background-color: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    padding: 1px 4px; font-size: 10px;
  }
  .create-note a { color: var(--theme-text-accent); }

  .create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-3);
  }
  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field.full { grid-column: 1 / -1; }
  .field-check { flex-direction: row; align-items: center; }
  .field-check label {
    display: flex; align-items: center; gap: var(--spacing-2);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
    cursor: pointer;
  }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input, .field select {
    font-family: var(--font-mono); font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field input[type="checkbox"] { width: auto; padding: 0; }
  .field input:focus, .field select:focus { outline: none; border-color: var(--theme-text-accent); }

  .form-actions {
    grid-column: 1 / -1;
    display: flex; align-items: center; gap: var(--spacing-3);
  }
  .save-btn {
    font-family: var(--font-display); font-size: 11px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: 2px solid var(--theme-text-accent); color: var(--theme-text-accent);
    cursor: pointer;
  }
  .save-btn:hover { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }
  .form-error { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-error); }

  .entity-table { display: flex; flex-direction: column; border: 3px solid var(--theme-border-primary); overflow-x: auto; -webkit-overflow-scrolling: touch; }
  .table-header, .table-row {
    display: grid;
    grid-template-columns: 50px 1fr 1fr 1fr 70px 90px 80px 60px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
  }
  .table-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
  }
  .table-row { border-bottom: 1px solid var(--theme-border-primary); }
  .table-row:last-child { border-bottom: none; }
  .table-row:hover { background-color: var(--theme-bg-hover); }

  .row-dex { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
  .row-slug { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }
  .row-name { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-primary); }
  .row-genus, .row-flags, .row-gen { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }

  .src-badge {
    font-family: var(--font-display); font-size: 8px;
    padding: 2px 6px; border: 2px solid var(--theme-border-primary); text-align: center;
  }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-link { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); text-decoration: none; }
  .edit-link:hover { text-decoration: underline; }
</style>
