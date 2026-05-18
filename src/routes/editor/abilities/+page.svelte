<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.abilities.filter(a =>
      a.slug.includes(query.toLowerCase()) || a.name.toLowerCase().includes(query.toLowerCase())
    )
  );
</script>

<svelte:head>
  <title>Edit Abilities | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <div class="page-header">
    <h2>ABILITIES</h2>
    <SearchBar bind:value={query} placeholder="Search abilities..." count={filtered.length} total={data.total} />
  </div>

  <div class="header-actions">
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW ABILITY'}
    </button>
  </div>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE ABILITY</h3>
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
          <label for="c-gen">GENERATION ID</label>
          <input id="c-gen" name="generationId" type="number" min="1" value={form?.createValues?.generationId ?? ''}>
        </div>
        <div class="field field-check">
          <label for="c-main">
            <input id="c-main" name="isMainSeries" type="checkbox" value="true"
                   checked={form?.createValues ? form.createValues.isMainSeries === 'true' || form.createValues.isMainSeries === 'on' : true}>
            MAIN SERIES
          </label>
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
      <p>No abilities yet.</p>
      <p class="hint">Click "+ NEW ABILITY" above to create the first one.</p>
    </div>
  {:else}
    <div class="abilities-grid">
      {#each filtered as a (a.id)}
        <div class="card-wrapper">
          <div class="ability-card card-interactive">
            <div class="ability-name">{a.name.toUpperCase().replace('-', ' ')}</div>
          </div>
          <a href="/editor/abilities/{a.slug}" class="card-link-overlay" aria-label="Edit {a.name}"></a>
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

  .field-check {
    flex-direction: row;
    align-items: center;
  }

  .field-check label {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    cursor: pointer;
  }

  .field-check input[type="checkbox"] {
    width: auto;
    padding: 0;
    accent-color: var(--theme-text-accent);
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

  .abilities-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .card-wrapper {
    position: relative;
  }

  .ability-card {
    padding: var(--spacing-4);
    min-height: 120px;
    display: flex;
    align-items: center;
  }

  .ability-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  .card-link-overlay {
    position: absolute;
    inset: 0;
    z-index: 1;
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom { color: var(--color-text-warning); border-color: var(--color-text-warning); }

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

  @media (max-width: 768px) {
    .abilities-grid { grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); }
  }
</style>
