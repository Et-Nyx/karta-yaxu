<script lang="ts">
  import { enhance } from '$app/forms';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import EffectText from '$lib/components/EffectText.svelte';
  import { pageSource } from '$lib/stores/pageSource';

  let { data, form } = $props();

  $effect(() => { pageSource.set(data.item.source); });

  let saved = $state(false);
  let savedEffects = $state(false);

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

  function handleSaveEffects() {
    savedEffects = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        savedEffects = true;
        setTimeout(() => savedEffects = false, 2500);
      }
    };
  }

  let nameVal = $state(data.item.name ?? '');
  let slugVal = $state(data.item.slug ?? '');
  let spriteUrl = $state(data.item.spriteUrl ?? '');
  let categoryVal = $state(data.item.category ?? '');

  $effect(() => {
    const src = form?.values ?? data.item;
    nameVal = (src.name as string) ?? '';
    slugVal = (src.slug as string) ?? '';
    spriteUrl = (src.spriteUrl as string) ?? '';
    categoryVal = (src.category as string) ?? '';
  });

  // Unique version groups that have a flavor text, preserving order
  let versions = $derived(() => {
    const seen = new Set<number>();
    return data.flavorTexts.filter((f: { versionGroupId: number }) => {
      if (seen.has(f.versionGroupId)) return false;
      seen.add(f.versionGroupId);
      return true;
    });
  });

  let selectedVersionId = $state<number | null>(null);

  $effect(() => {
    const vs = versions();
    if (selectedVersionId === null && vs.length > 0) {
      const frlg = vs.find((v: { versionGroupName: string }) => v.versionGroupName === 'firered-leafgreen');
      selectedVersionId = (frlg ?? vs[vs.length - 1]).versionGroupId;
    }
  });

  let currentFlavorText = $derived(() => {
    if (!selectedVersionId) return null;
    return data.flavorTexts.find((f: { versionGroupId: number; flavorText: string }) => f.versionGroupId === selectedVersionId)?.flavorText ?? null;
  });
</script>

<svelte:head>
  <title>Edit {data.item.name} | Karta Yaxu</title>
</svelte:head>

<div class="item-detail">
  <form action="?/update" method="post" use:enhance={handleSave}>
    <header class="detail-header">
      <div class="header-row">
        {#if spriteUrl}
          <img src={spriteUrl} alt={nameVal} class="header-sprite" style="image-rendering: pixelated;" />
        {/if}
        <div class="header-info">
          <input class="h1-input" name="name" bind:value={nameVal} maxlength="100" required />
          {#if categoryVal}
            <span class="category-badge">{categoryVal.replace(/-/g, ' ')}</span>
          {/if}
        </div>
      </div>
      <div class="meta-strip">
        <div class="meta-field"><label>SLUG</label><input name="slug" type="text" bind:value={slugVal} pattern="[a-z0-9-]+" required></div>
        <div class="meta-field"><label>SPRITE URL</label><input name="spriteUrl" type="text" bind:value={spriteUrl}></div>
        {#if data.item.pokeapiId}<span class="meta-id">PokéAPI: {data.item.pokeapiId}</span>{/if}
        <div class="meta-actions">
          <button type="submit" class="save-btn">SAVE</button>
          <a href="." class="cancel-link">CANCEL</a>
          {#if saved}<span class="save-ok">✓ SAVED</span>{/if}
          {#if form?.updateError}<span class="form-error">{form.updateError}</span>{/if}
        </div>
      </div>
    </header>

    <div class="detail-grid">
      <!-- Left: data + flavor text -->
      <div class="left-col">
        <InfoBox title="DATA">
          <div class="data-row">
            <span>BUY PRICE</span>
            <input name="cost" type="number" min="0" value={data.item.cost ?? ''} />
          </div>
          <div class="data-row">
            <span>FLING POWER</span>
            <input name="flingPower" type="number" min="0" value={data.item.flingPower ?? ''} />
          </div>
          <div class="data-row">
            <span>CATEGORY</span>
            <input name="category" type="text" bind:value={categoryVal} />
          </div>
        </InfoBox>

        {#if versions().length > 0}
          <VersionSelector
            bind:value={selectedVersionId}
            options={versions().map((v: { versionGroupId: number; versionGroupName: string }) => ({ value: v.versionGroupId, label: v.versionGroupName.toUpperCase().replace(/-/g, ' ') }))}
          />
          {#if currentFlavorText()}
            <FlavorTextBox text={currentFlavorText()!} />
          {/if}
        {/if}
      </div>

      <!-- Right: effect -->
      {#if data.effect}
        <div class="right-col">
          <InfoBox title="EFFECT">
            <EffectText shortEffect={data.effect.shortEffect} effect={data.effect.effect} />
          </InfoBox>
        </div>
      {/if}
    </div>
  </form>

  {#if data.effect}
    <form action="?/updateEffects" method="post" use:enhance={handleSaveEffects} class="effects-form">
      <InfoBox title="EDIT EFFECT">
        <div class="effect-fields">
          <div class="effect-field">
            <label>SHORT EFFECT</label>
            <textarea name="shortEffect" rows="3">{data.effect.shortEffect}</textarea>
          </div>
          <div class="effect-field">
            <label>FULL EFFECT</label>
            <textarea name="effect" rows="6">{data.effect.effect}</textarea>
          </div>
          <div class="effect-actions">
            <button type="submit" class="save-btn">SAVE EFFECT</button>
            {#if savedEffects}<span class="save-ok">✓ SAVED</span>{/if}
            {#if form?.updateError}<span class="form-error">{form.updateError}</span>{/if}
          </div>
        </div>
      </InfoBox>
    </form>
  {/if}

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
  .item-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }


  /* ── Header ── */

  .detail-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: var(--shadow-card);
  }

  .header-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-6);
  }

  .header-sprite {
    width: 96px;
    height: 96px;
    object-fit: contain;
    flex-shrink: 0;
    image-rendering: pixelated;
  }

  .header-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .h1-input {
    font-family: var(--font-display);
    font-size: 20px;
    color: var(--theme-text-primary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    outline: none;
    padding: 0;
  }

  .h1-input:focus {
    border-bottom-color: var(--theme-text-accent);
  }

  .category-badge {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    text-transform: capitalize;
  }

  /* ── Meta strip ── */

  .meta-strip {
    border-top: 2px dashed var(--theme-border-primary);
    padding-top: var(--spacing-3);
    display: flex;
    gap: var(--spacing-4);
    align-items: center;
    flex-wrap: wrap;
  }

  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .meta-field input {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    padding: 2px var(--spacing-1);
  }

  .meta-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    margin-left: auto;
  }

  .save-btn {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-1) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
  }

  .save-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .cancel-link {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-muted);
    text-decoration: none;
  }

  .cancel-link:hover {
    text-decoration: underline;
  }

  .save-ok {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-accent);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
  }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .meta-id {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .wiki-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    text-decoration: none;
  }

  .wiki-link:hover {
    text-decoration: underline;
  }

  /* ── Grid ── */

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
    align-items: start;
  }

  .left-col, .right-col {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  /* ── Data rows ── */

  .data-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-1) 0;
    border-bottom: 1px solid var(--theme-border-primary);
    gap: var(--spacing-2);
  }

  .data-row:last-child {
    border-bottom: none;
  }

  .data-row span {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .data-row input {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-primary);
    background: none;
    border: none;
    border-bottom: 1px solid var(--theme-border-primary);
    text-align: right;
    width: 80px;
    outline: none;
    padding: 2px 0;
  }

  .data-row input:focus {
    border-bottom-color: var(--theme-text-accent);
  }

  /* ── Effects form ── */

  .effects-form {
    display: flex;
    flex-direction: column;
  }

  .effect-fields {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .effect-field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .effect-field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .effect-field textarea {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    padding: var(--spacing-2);
    resize: vertical;
  }

  .effect-field textarea:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .effect-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  /* ── Resource / delete forms ── */

  .resource-form, .delete-form {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .resource-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--theme-text-muted);
    color: var(--theme-text-muted);
    cursor: pointer;
  }

  .resource-btn:hover {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .resource-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
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

  @media (max-width: 768px) {
    .detail-header {
      gap: var(--spacing-3);
    }

    .header-row {
      flex-direction: column;
      align-items: center;
      text-align: center;
    }

    .detail-grid {
      grid-template-columns: minmax(0, 1fr);
    }

    .left-col, .right-col { min-width: 0; }

    .meta-strip {
      flex-direction: column;
      align-items: flex-start;
    }

    .meta-actions {
      margin-left: 0;
    }
  }
</style>
