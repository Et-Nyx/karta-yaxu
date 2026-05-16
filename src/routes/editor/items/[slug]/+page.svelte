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

  let item = $derived(form?.values ?? data.item);
</script>

<svelte:head>
  <title>Edit {data.item.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <a href="/editor/items" class="back">← BACK</a>
      <h2>EDIT ITEM: {data.item.name.toUpperCase()}</h2>
      <span class="src-badge src-{data.item.source}">{data.item.source}</span>
    </div>
    <div class="meta">
      <span>ID: {data.item.id}</span>
      {#if data.item.pokeapiId}<span>PokéAPI: {data.item.pokeapiId}</span>{/if}
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="100" required value={item.name ?? ''}>
    </div>

    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="100" pattern="[a-z0-9-]+" required value={item.slug ?? ''}>
    </div>

    <div class="field">
      <label for="cost">COST</label>
      <input id="cost" name="cost" type="number" min="0" value={item.cost ?? ''}>
    </div>

    <div class="field">
      <label for="fling">FLING POWER</label>
      <input id="fling" name="flingPower" type="number" min="0" value={item.flingPower ?? ''}>
    </div>

    <div class="field">
      <label for="cat">CATEGORY</label>
      <input id="cat" name="category" type="text" placeholder="berry / medicine / ball / ..." value={item.category ?? ''}>
    </div>

    <div class="field sprite-field">
      <label for="sprite">SPRITE URL</label>
      <input id="sprite" name="spriteUrl" type="text" placeholder="https://..." value={item.spriteUrl ?? ''}>
      {#if item.spriteUrl}
        <img class="sprite-preview" src={item.spriteUrl} alt={item.name ?? ''} />
      {/if}
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

  {#if data.item.source === 'edited' && data.item.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import this item from PokeAPI? All your edits will be lost.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Reverts to canonical PokeAPI data; source flips back to 'pokeapi'.</span>
    </form>
  {/if}

  {#if data.item.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS ITEM</button>
      <span class="delete-hint">Apenas registros custom podem ser deletados.</span>
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

  .back:hover {
    text-decoration: underline;
  }

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

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
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

  .sprite-preview {
    margin-top: var(--spacing-2);
    max-height: 64px;
    max-width: 100%;
    object-fit: contain;
    image-rendering: pixelated;
    align-self: flex-start;
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
