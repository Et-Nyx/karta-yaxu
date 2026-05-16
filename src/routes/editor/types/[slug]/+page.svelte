<script lang="ts">
  import { enhance } from '$app/forms';

  let { data, form } = $props();

  const DMG_CLASS_OPTIONS = ['', 'physical', 'special', 'status'];

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

  let type = $derived(form?.values ?? data.type);
</script>

<svelte:head>
  <title>Edit {data.type.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <a href="/editor/types" class="back">← BACK</a>
      <h2>EDIT TYPE: {data.type.name.toUpperCase()}</h2>
      <span class="src-badge src-{data.type.source}">{data.type.source}</span>
    </div>
    <div class="meta">
      <span>ID: {data.type.id}</span>
      {#if data.type.pokeapiId}<span>PokéAPI: {data.type.pokeapiId}</span>{/if}
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="50" required value={type.name ?? ''}>
    </div>

    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="50" pattern="[a-z0-9-]+" required value={type.slug ?? ''}>
    </div>

    <div class="field">
      <label for="hex">HEX COLOR</label>
      <div class="hex-row">
        {#if type.hexColor}
          <span class="swatch" style="background-color: {type.hexColor}"></span>
        {/if}
        <input id="hex" name="hexColor" type="text" placeholder="#FF0000" value={type.hexColor ?? ''}>
      </div>
    </div>

    <div class="field">
      <label for="icon">ICON URL</label>
      <input id="icon" name="iconUrl" type="text" placeholder="/icons/types/normal.png" value={type.iconUrl ?? ''}>
    </div>

    <div class="field">
      <label for="dmg">DMG CLASS</label>
      <select id="dmg" name="moveDamageClass">
        {#each DMG_CLASS_OPTIONS as v}
          <option value={v} selected={(type.moveDamageClass ?? '') === v}>{v || '— none —'}</option>
        {/each}
      </select>
    </div>

    <div class="field">
      <label for="gen">GENERATION ID</label>
      <input id="gen" name="generationId" type="number" min="1" value={type.generationId ?? ''}>
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

  {#if data.type.source === 'edited' && data.type.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import this type from PokeAPI? All your edits will be lost.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Reverts to canonical PokeAPI data; source flips back to 'pokeapi'.</span>
    </form>
  {/if}

  {#if data.type.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS TYPE</button>
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

  .hex-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .hex-row input {
    flex: 1;
  }

  .swatch {
    display: inline-block;
    width: 22px;
    height: 22px;
    border: 2px solid var(--theme-border-primary);
    flex-shrink: 0;
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
