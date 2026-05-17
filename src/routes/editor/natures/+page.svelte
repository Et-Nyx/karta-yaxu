<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.natures.filter(n =>
      n.slug.includes(query.toLowerCase()) || n.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  const STAT_OPTIONS = ['', 'attack', 'defense', 'special-attack', 'special-defense', 'speed'];
  const FLAVOR_OPTIONS = ['', 'spicy', 'sour', 'sweet', 'bitter', 'dry'];
</script>

<svelte:head>
  <title>Edit Natures | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>NATURES</h2>
      <span class="count">{data.total} total · {filtered.length} shown</span>
    </div>
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW NATURE'}
    </button>
  </header>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE NATURE</h3>
      <form action="?/create" method="post" use:enhance class="create-form">
        <div class="field">
          <label for="c-name">NAME</label>
          <input id="c-name" name="name" type="text" maxlength="50" required value={form?.createValues?.name ?? ''}>
        </div>
        <div class="field">
          <label for="c-slug">SLUG</label>
          <input id="c-slug" name="slug" type="text" maxlength="50" pattern="[a-z0-9-]+" required value={form?.createValues?.slug ?? ''}>
        </div>
        <div class="field">
          <label for="c-inc">INCREASED STAT</label>
          <select id="c-inc" name="increasedStat">
            {#each STAT_OPTIONS as v}
              <option value={v} selected={form?.createValues?.increasedStat === v}>{v || '—'}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="c-dec">DECREASED STAT</label>
          <select id="c-dec" name="decreasedStat">
            {#each STAT_OPTIONS as v}
              <option value={v} selected={form?.createValues?.decreasedStat === v}>{v || '—'}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="c-likes">LIKES FLAVOR</label>
          <select id="c-likes" name="likesFlavor">
            {#each FLAVOR_OPTIONS as v}
              <option value={v} selected={form?.createValues?.likesFlavor === v}>{v || '—'}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="c-hates">HATES FLAVOR</label>
          <select id="c-hates" name="hatesFlavor">
            {#each FLAVOR_OPTIONS as v}
              <option value={v} selected={form?.createValues?.hatesFlavor === v}>{v || '—'}</option>
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

  <SearchBar bind:value={query} placeholder="Search natures..." count={filtered.length} total={data.total} />

  <div class="entity-table">
    <div class="table-header">
      <span>SLUG</span>
      <span>NAME</span>
      <span>↑ STAT</span>
      <span>↓ STAT</span>
      <span>FLAVOR</span>
      <span>SOURCE</span>
      <span></span>
    </div>
    {#each filtered as n (n.id)}
      <div class="table-row">
        <span class="row-slug">{n.slug}</span>
        <span class="row-name">{n.name}</span>
        <span class="row-stat">{n.increasedStat ?? '—'}</span>
        <span class="row-stat">{n.decreasedStat ?? '—'}</span>
        <span class="row-flavor">{n.likesFlavor ?? '—'} / {n.hatesFlavor ?? '—'}</span>
        <span class="src-badge src-{n.source}">{n.source}</span>
        <a href="/editor/natures/{n.slug}" class="edit-link">EDIT →</a>
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

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

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

  .entity-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 1fr 1fr 100px 100px 1fr 80px 60px;
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

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background-color: var(--theme-bg-hover);
  }

  .row-slug {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
  }

  .row-name {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .row-stat,
  .row-flavor {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
    text-align: center;
  }

  .src-pokeapi {
    color: var(--theme-text-muted);
  }

  .src-edited {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .src-custom {
    color: var(--color-text-warning);
    border-color: var(--color-text-warning);
  }

  .edit-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    text-decoration: none;
  }

  .edit-link:hover {
    text-decoration: underline;
  }
</style>
