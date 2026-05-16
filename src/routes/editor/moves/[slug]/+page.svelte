<script lang="ts">
  import { enhance } from '$app/forms';

  let { data, form } = $props();

  let saved = $state(false);
  let effectsSaved = $state(false);

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

  function handleEffectsSave() {
    effectsSaved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        effectsSaved = true;
        setTimeout(() => effectsSaved = false, 2500);
      }
    };
  }

  let move = $derived(form?.values ?? data.move);
</script>

<svelte:head>
  <title>Edit {data.move.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <a href="/editor/moves" class="back">← BACK</a>
      <h2>EDIT MOVE: {data.move.name.toUpperCase()}</h2>
      <span class="src-badge src-{data.move.source}">{data.move.source}</span>
    </div>
    <div class="meta">
      <span>ID: {data.move.id}</span>
      {#if data.move.pokeapiId}<span>PokéAPI: {data.move.pokeapiId}</span>{/if}
      <a href="/wiki/moves/{data.move.slug}" class="wiki-link">VIEW IN WIKI →</a>
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="80" required value={move.name ?? ''}>
    </div>
    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required value={move.slug ?? ''}>
    </div>
    <div class="field">
      <label for="type">TYPE</label>
      <select id="type" name="typeId" required>
        {#each data.types as t}
          <option value={t.id} selected={(move.typeId ?? null) === t.id}>{t.name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="dmg">DAMAGE CLASS</label>
      <select id="dmg" name="damageClass" required>
        {#each ['physical', 'special', 'status'] as v}
          <option value={v} selected={(move.damageClass ?? '') === v}>{v}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="power">POWER</label>
      <input id="power" name="power" type="number" min="0" value={move.power ?? ''}>
    </div>
    <div class="field">
      <label for="acc">ACCURACY</label>
      <input id="acc" name="accuracy" type="number" min="0" max="100" value={move.accuracy ?? ''}>
    </div>
    <div class="field">
      <label for="pp">PP</label>
      <input id="pp" name="pp" type="number" min="0" required value={move.pp ?? ''}>
    </div>
    <div class="field">
      <label for="priority">PRIORITY</label>
      <input id="priority" name="priority" type="number" value={move.priority ?? 0}>
    </div>
    <div class="field">
      <label for="target">TARGET</label>
      <input id="target" name="target" type="text" value={move.target ?? ''} placeholder="selected-pokemon / all-opponents / ...">
    </div>
    <div class="field">
      <label for="gen">GENERATION</label>
      <select id="gen" name="generationId">
        <option value="">— none —</option>
        {#each data.generations as g}
          <option value={g.id} selected={(move.generationId ?? null) === g.id}>{g.name}</option>
        {/each}
      </select>
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

  <!-- English effects sub-form -->
  <section class="effects-section">
    <h3 class="section-title">ENGLISH EFFECTS</h3>
    <form action="?/updateEffects" method="post" use:enhance={handleEffectsSave} class="effects-form">
      <div class="field">
        <label for="effect">EFFECT</label>
        <textarea id="effect" name="effect" rows="4" maxlength="4000">{data.effects?.effect ?? ''}</textarea>
      </div>
      <div class="field">
        <label for="short">SHORT EFFECT</label>
        <textarea id="short" name="shortEffect" rows="2" maxlength="2000">{data.effects?.shortEffect ?? ''}</textarea>
      </div>
      <div class="form-actions">
        <button type="submit" class="save-btn">SAVE EFFECTS</button>
        {#if form?.effectsError}
          <span class="form-error">{form.effectsError}</span>
        {/if}
        {#if effectsSaved}
          <span class="save-ok">✓ SAVED</span>
        {/if}
      </div>
    </form>
  </section>

  {#if data.move.source === 'edited' && data.move.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import this move from PokeAPI? All your edits will be lost.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Reverts to canonical PokeAPI data; source flips back to 'pokeapi'.</span>
    </form>
  {/if}

  {#if data.move.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS MOVE</button>
      <span class="delete-hint">Apenas moves custom podem ser deletados.</span>
    </form>
  {/if}
</div>

<style>
  .edit-page { display: flex; flex-direction: column; gap: var(--spacing-5); }
  .page-header {
    display: flex; justify-content: space-between; align-items: flex-end;
    gap: var(--spacing-3); flex-wrap: wrap;
  }
  .header-left { display: flex; align-items: center; gap: var(--spacing-3); flex-wrap: wrap; }
  .back { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); text-decoration: none; }
  .back:hover { text-decoration: underline; }
  .page-header h2 { color: var(--theme-text-primary); margin: 0; }
  .meta { display: flex; gap: var(--spacing-3); font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
  .wiki-link { color: var(--theme-text-accent); text-decoration: none; }
  .wiki-link:hover { text-decoration: underline; }

  .src-badge { font-family: var(--font-display); font-size: 8px; padding: 2px 6px; border: 2px solid var(--theme-border-primary); }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-form, .effects-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .effects-form { grid-template-columns: 1fr; }

  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input, .field select, .field textarea {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field textarea { resize: vertical; }
  .field input:focus, .field select:focus, .field textarea:focus {
    outline: none; border-color: var(--theme-text-accent);
  }

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
  .save-ok { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-accent); }

  .effects-section { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .section-title {
    font-family: var(--font-display); font-size: 11px; color: var(--theme-text-secondary);
    margin: 0; padding-bottom: var(--spacing-2); border-bottom: 2px solid var(--theme-border-primary);
  }

  .delete-form { display: flex; align-items: center; gap: var(--spacing-3); }
  .delete-btn {
    font-family: var(--font-display); font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: 2px solid var(--color-text-error); color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }
  .delete-hint { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
</style>
