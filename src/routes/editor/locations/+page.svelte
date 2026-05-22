<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.locations.filter(l =>
      l.slug.includes(query.toLowerCase()) ||
      l.name.toLowerCase().includes(query.toLowerCase()) ||
      (l.regionName ?? '').toLowerCase().includes(query.toLowerCase())
    )
  );

  function autoSlug(name: string): string {
    return name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
  }

  let nameVal = $state('');
  let slugVal = $state('');
  let slugTouched = $state(false);

  $effect(() => {
    if (!slugTouched) slugVal = autoSlug(nameVal);
  });
</script>

<svelte:head>
  <title>Edit Locations | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>LOCATIONS</h2>
    </div>
    <button type="button" class="new-btn" onclick={() => { showCreate = !showCreate; nameVal = ''; slugVal = ''; slugTouched = false; }}>
      {showCreate ? '× CANCEL' : '+ NEW LOCATION'}
    </button>
  </header>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE LOCATION</h3>
      <form action="?/create" method="post" use:enhance class="create-form">
        <div class="field">
          <label for="c-name">NAME</label>
          <input id="c-name" name="name" type="text" maxlength="100" required
            bind:value={nameVal}>
        </div>
        <div class="field">
          <label for="c-slug">SLUG</label>
          <input id="c-slug" name="slug" type="text" maxlength="100" pattern="[a-z0-9-]+" required
            bind:value={slugVal}
            oninput={() => slugTouched = true}>
        </div>
        <div class="field">
          <label for="c-region">REGION</label>
          <select id="c-region" name="regionId">
            <option value="">—</option>
            {#each data.regions as r}
              <option value={r.id} selected={form?.createValues?.regionId == String(r.id)}>
                {r.name}
              </option>
            {/each}
          </select>
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

  <SearchBar bind:value={query} placeholder="Search locations..." count={filtered.length} total={data.total} />

  <div class="entity-table">
    <div class="table-header">
      <span>SLUG</span>
      <span>NAME</span>
      <span>REGION</span>
      <span class="right">AREAS</span>
      <span>SOURCE</span>
      <span></span>
    </div>
    {#each filtered as l (l.id)}
      <div class="table-row">
        <span class="row-slug">{l.slug}</span>
        <span class="row-name">{l.name}</span>
        <span class="row-region">{l.regionName ?? '—'}</span>
        <span class="row-areas right">{l.areaCount}</span>
        <span class="src-badge src-{l.source}">{l.source}</span>
        <a href="/editor/locations/{l.slug}" class="edit-link">EDIT →</a>
      </div>
    {/each}
  </div>
</div>

<style>
  .entity-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
  }

  .page-header h2 { color: var(--theme-text-primary); margin: 0; }

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .new-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    cursor: pointer;
  }
  .new-btn:hover { border-color: var(--theme-text-accent); color: var(--theme-text-accent); }

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

  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }

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
  .field select:focus { outline: none; border-color: var(--theme-text-accent); }

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
  .save-btn:hover { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }

  .form-error { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-error); }

  .entity-table {
    display: flex;
    flex-direction: column;
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    overflow-x: auto;
    background-color: var(--theme-bg-secondary);
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 1fr 1fr 120px 60px 80px 60px;
    gap: var(--spacing-3);
    padding: var(--spacing-3) var(--spacing-4);
    align-items: center;
  }

  .table-header {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 3px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-inverse);
  }

  .table-row {
    border-bottom: 1px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    transition: background-color 0.05s ease;
  }
  .table-row:nth-child(even) {
    background-color: var(--theme-bg-primary);
  }
  .table-row:last-child { border-bottom: none; }
  .table-row:hover { background-color: var(--theme-bg-hover); }

  .row-slug { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }
  .row-name { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-primary); }
  .row-region { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .row-areas { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .right { text-align: right; }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
    text-align: center;
  }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-link { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); text-decoration: none; }
  .edit-link:hover { text-decoration: underline; }
</style>
