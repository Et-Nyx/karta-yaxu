<script lang="ts">
  import { enhance } from '$app/forms';

  let { data, form } = $props();

  let saved = $state(false);

  function handleSave() {
    saved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        saved = true;
        setTimeout(() => saved = false, 2500);
      }
    };
  }

  let pokedex = $derived(form?.values ?? data.pokedex);
</script>

<svelte:head>
  <title>Edit {data.pokedex.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <a href="/editor/pokedex" class="back">← BACK</a>
      <h2>EDIT POKÉDEX: {data.pokedex.name.toUpperCase()}</h2>
      <span class="src-badge src-{data.pokedex.source}">{data.pokedex.source}</span>
    </div>
    <div class="meta">
      <span>ID: {data.pokedex.id}</span>
      {#if data.pokedex.pokeapiId}<span>PokéAPI: {data.pokedex.pokeapiId}</span>{/if}
      <a href="/wiki/pokedex/{data.pokedex.slug}" class="wiki-link">VIEW IN WIKI →</a>
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="80" required value={pokedex.name ?? ''}>
    </div>
    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required value={pokedex.slug ?? ''}>
    </div>
    <div class="field">
      <label for="region">REGION</label>
      <select id="region" name="regionId">
        <option value="">— none —</option>
        {#each data.regions as r}
          <option value={r.id} selected={(pokedex.regionId ?? null) === r.id}>{r.name}</option>
        {/each}
      </select>
    </div>
    <div class="field field-check">
      <label for="main">
        <input id="main" name="isMainSeries" type="checkbox" value="true" checked={pokedex.isMainSeries}>
        MAIN SERIES
      </label>
    </div>
    <div class="form-actions">
      <button type="submit" class="save-btn">SAVE</button>
      {#if form?.updateError}
        <span class="form-error">{form.updateError}</span>
      {/if}
      {#if saved}
        <span class="save-ok">✓ SAVED</span>
      {/if}
    </div>
  </form>

  <!-- Entries sub-editor -->
  <section class="entries-section">
    <h3 class="section-title">ENTRIES ({data.entries.length})</h3>

    <form action="?/addEntry" method="post" use:enhance class="add-entry-form">
      <div class="field">
        <label for="add-species">SPECIES</label>
        <select id="add-species" name="speciesId" required>
          <option value="">— pick a species —</option>
          {#each data.species as s}
            <option value={s.id}>{s.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for="add-num">ENTRY #</label>
        <input id="add-num" name="entryNumber" type="number" min="0" max="9999" required>
      </div>
      <div class="add-actions">
        <button type="submit" class="add-btn">+ ADD ENTRY</button>
        {#if form?.entryError}
          <span class="form-error">{form.entryError}</span>
        {/if}
      </div>
    </form>

    {#if data.entries.length === 0}
      <div class="empty-state">
        <p>No entries yet.</p>
        <p class="hint">Add species using the form above. Entries are ordered by entry number.</p>
      </div>
    {:else}
      <div class="entries-table">
        <div class="table-header">
          <span>#</span>
          <span>SPECIES</span>
          <span></span>
        </div>
        {#each data.entries as e (e.speciesId)}
          <div class="table-row">
            <form action="?/updateEntry" method="post" use:enhance class="num-form">
              <input type="hidden" name="speciesId" value={e.speciesId}>
              <input
                name="entryNumber"
                type="number"
                min="0"
                max="9999"
                value={e.entryNumber}
                onchange={(ev) => (ev.currentTarget.form as HTMLFormElement).requestSubmit()}
              >
            </form>
            <a href="/wiki/pokemon/{e.speciesSlug}" class="species-link">{e.speciesName}</a>
            <form action="?/removeEntry" method="post" use:enhance class="remove-form">
              <input type="hidden" name="speciesId" value={e.speciesId}>
              <button type="submit" class="remove-btn">REMOVE</button>
            </form>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  {#if data.pokedex.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS POKÉDEX</button>
      <span class="delete-hint">Apenas pokédexes custom podem ser deletadas. Todas as entries são removidas junto.</span>
    </form>
  {/if}
</div>

<style>
  .edit-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-5);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: var(--spacing-3);
    flex-wrap: wrap;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    flex-wrap: wrap;
  }

  .back {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    text-decoration: none;
  }

  .back:hover { text-decoration: underline; }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .meta {
    display: flex;
    gap: var(--spacing-3);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .wiki-link {
    color: var(--theme-text-accent);
    text-decoration: none;
  }

  .wiki-link:hover { text-decoration: underline; }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .field-check {
    flex-direction: row;
    align-items: center;
  }

  .field-check label {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    cursor: pointer;
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

  .field input[type="checkbox"] { width: auto; padding: 0; }

  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--theme-text-accent);
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

  .save-ok {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-accent);
  }

  /* Entries sub-editor */
  .entries-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .add-entry-form {
    display: grid;
    grid-template-columns: 1fr 120px auto;
    gap: var(--spacing-3);
    align-items: end;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
  }

  .add-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .add-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
    white-space: nowrap;
  }

  .add-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .entries-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 80px 1fr 80px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
  }

  .table-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .table-row {
    border-bottom: 1px solid var(--theme-border-primary);
  }

  .table-row:last-child { border-bottom: none; }

  .num-form input {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: var(--spacing-1);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    width: 60px;
  }

  .num-form input:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .species-link {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
    text-decoration: none;
  }

  .species-link:hover {
    color: var(--theme-text-accent);
    text-decoration: underline;
  }

  .remove-btn {
    font-family: var(--font-display);
    font-size: 9px;
    padding: 2px 6px;
    background: none;
    border: 2px solid var(--color-text-error);
    color: var(--color-text-error);
    cursor: pointer;
  }

  .remove-btn:hover {
    background-color: var(--color-text-error);
    color: var(--theme-bg-primary);
  }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-5);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-1) 0;
  }

  .empty-state .hint {
    font-size: 11px;
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
</style>
