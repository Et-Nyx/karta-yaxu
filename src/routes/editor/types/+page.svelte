<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import TypeIcon from '$lib/components/TypeIcon.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.types.filter(t =>
      t.slug.includes(query.toLowerCase()) || t.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  const DMG_CLASS_OPTIONS = ['', 'physical', 'special', 'status'];
</script>

<svelte:head>
  <title>Edit Types | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <div class="page-header">
    <h2>TYPES</h2>
    <SearchBar bind:value={query} placeholder="Search types..." count={filtered.length} total={data.total} />
  </div>

  <div class="header-actions">
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW TYPE'}
    </button>
  </div>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE TYPE</h3>
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
          <label for="c-hex">HEX COLOR</label>
          <input id="c-hex" name="hexColor" type="text" placeholder="#FF0000" value={form?.createValues?.hexColor ?? ''}>
        </div>
        <div class="field">
          <label for="c-icon">ICON URL</label>
          <input id="c-icon" name="iconUrl" type="text" placeholder="/icons/types/normal.png" value={form?.createValues?.iconUrl ?? ''}>
        </div>
        <div class="field">
          <label for="c-dmg">DMG CLASS</label>
          <select id="c-dmg" name="moveDamageClass">
            {#each DMG_CLASS_OPTIONS as v}
              <option value={v} selected={form?.createValues?.moveDamageClass === v}>{v || '—'}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="c-gen">GENERATION ID</label>
          <input id="c-gen" name="generationId" type="number" min="1" value={form?.createValues?.generationId ?? ''}>
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
      <p>No types yet.</p>
      <p class="hint">Click "+ NEW TYPE" above to create the first one.</p>
    </div>
  {:else}
    <div class="types-grid">
      {#each filtered as t (t.id)}
        <div class="card-wrapper">
          <div class="type-card card-interactive">
            <div class="type-content">
              <TypeIcon typeName={t.name} size="lg" />
              <span class="type-name">{t.name.toUpperCase()}</span>
            </div>
            <div class="type-bar" style="background-color: {t.hexColor ?? `var(--color-type-${t.name})`}"></div>
          </div>
          <a href="/editor/types/{t.slug}" class="card-link-overlay" aria-label="Edit {t.name}"></a>
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

  .types-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .card-wrapper {
    position: relative;
  }

  .type-card {
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
    text-align: center;
    min-height: 120px;
  }

  .type-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-2);
    padding: var(--spacing-4) var(--spacing-4) var(--spacing-3);
    flex: 1;
  }

  .type-name {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    letter-spacing: 1px;
  }

  .type-bar {
    height: 4px;
    width: 100%;
    flex-shrink: 0;
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

  @media (max-width: 768px) {
    .types-grid {
      grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    }

    .type-content {
      padding: var(--spacing-3) var(--spacing-2) var(--spacing-2);
    }

    .type-name {
      font-size: 8px;
    }
  }
</style>
