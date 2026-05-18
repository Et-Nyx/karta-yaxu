<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import ItemCard from '$lib/components/ItemCard.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.items.filter(i =>
      i.slug.includes(query.toLowerCase()) ||
      i.name.toLowerCase().includes(query.toLowerCase()) ||
      (i.category?.toLowerCase().includes(query.toLowerCase()) ?? false)
    )
  );
</script>

<svelte:head>
  <title>Edit Items | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <div class="page-header">
    <h2>ITEMS</h2>
    <SearchBar bind:value={query} placeholder="Search items..." count={filtered.length} total={data.total} />
  </div>

  <div class="header-actions">
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW ITEM'}
    </button>
  </div>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE ITEM</h3>
      <form action="?/create" method="post" use:enhance class="create-form">
        <div class="field">
          <label for="c-name">NAME</label>
          <input id="c-name" name="name" type="text" maxlength="100" required value={form?.createValues?.name ?? ''}>
        </div>
        <div class="field">
          <label for="c-slug">SLUG</label>
          <input id="c-slug" name="slug" type="text" maxlength="100" pattern="[a-z0-9-]+" required value={form?.createValues?.slug ?? ''}>
        </div>
        <div class="field">
          <label for="c-cost">COST</label>
          <input id="c-cost" name="cost" type="number" min="0" value={form?.createValues?.cost ?? ''}>
        </div>
        <div class="field">
          <label for="c-fling">FLING POWER</label>
          <input id="c-fling" name="flingPower" type="number" min="0" value={form?.createValues?.flingPower ?? ''}>
        </div>
        <div class="field">
          <label for="c-cat">CATEGORY</label>
          <input id="c-cat" name="category" type="text" placeholder="berry / medicine / ball / ..." value={form?.createValues?.category ?? ''}>
        </div>
        <div class="field">
          <label for="c-sprite">SPRITE URL</label>
          <input id="c-sprite" name="spriteUrl" type="text" placeholder="https://..." value={form?.createValues?.spriteUrl ?? ''}>
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
      <p>No items yet.</p>
      <p class="hint">Click "+ NEW ITEM" above to create the first one.</p>
    </div>
  {:else}
    <div class="items-grid">
      {#each filtered as i (i.id)}
        <div class="card-wrapper">
          <ItemCard
            name={i.name}
            slug={i.slug}
            spriteUrl={i.spriteUrl}
            category={i.category}
            cost={i.cost}
          />
          <a href="/editor/items/{i.slug}" class="card-link-overlay" aria-label="Edit {i.name}"></a>
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

  .field input {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }

  .field input:focus {
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

  .items-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
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
