<script lang="ts">
  import { enhance } from '$app/forms';
  import { pageSource } from '$lib/stores/pageSource';

  let { data, form } = $props();

  $effect(() => { pageSource.set(data.location.source); });

  let saved = $state(false);
  let showCreateArea = $state(false);
  let showCreateEncounter = $state(false);

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

  let loc = $derived(form?.values ?? data.location);

  function autoSlug(name: string): string {
    return name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
  }

  let areaNameVal = $state('');
  let areaSlugVal = $state('');
  let areaSlugTouched = $state(false);

  $effect(() => {
    if (!areaSlugTouched) areaSlugVal = autoSlug(areaNameVal);
  });
</script>

<svelte:head>
  <title>Edit {data.location.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <h2>EDIT LOCATION: {data.location.name.toUpperCase()}</h2>
    </div>
    <div class="meta">
      <span>ID: {data.location.id}</span>
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="100" required value={loc.name ?? ''}>
    </div>
    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="100" pattern="[a-z0-9-]+" required value={loc.slug ?? ''}>
    </div>
    <div class="field">
      <label for="region">REGION</label>
      <select id="region" name="regionId">
        <option value="">—</option>
        {#each data.regions as r}
          <option value={r.id} selected={(loc.regionId ?? null) === r.id}>{r.name}</option>
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

  <!-- Areas section -->
  <section class="areas-section">
    <div class="section-header">
      <h3 class="section-title">AREAS ({data.areas.length})</h3>
      <button type="button" class="new-btn" onclick={() => { showCreateArea = !showCreateArea; areaNameVal = ''; areaSlugVal = ''; areaSlugTouched = false; }}>
        {showCreateArea ? '× CANCEL' : '+ NEW AREA'}
      </button>
    </div>

    {#if showCreateArea}
      <form action="?/createArea" method="post" use:enhance class="area-create-form">
        <div class="field">
          <label for="a-name">NAME</label>
          <input id="a-name" name="name" type="text" maxlength="100" required
            bind:value={areaNameVal}>
        </div>
        <div class="field">
          <label for="a-slug">SLUG</label>
          <input id="a-slug" name="slug" type="text" maxlength="100" pattern="[a-z0-9-]+" required
            bind:value={areaSlugVal}
            oninput={() => areaSlugTouched = true}>
        </div>
        <div class="field">
          <label for="a-idx">GAME INDEX</label>
          <input id="a-idx" name="gameIndex" type="number" min="0"
            value={form?.areaValues?.gameIndex ?? ''}>
        </div>
        <div class="form-actions">
          <button type="submit" class="save-btn">CREATE AREA</button>
          {#if form?.areaError}
            <span class="form-error">{form.areaError}</span>
          {/if}
        </div>
      </form>
    {/if}

    {#if data.areas.length > 0}
      <div class="areas-table">
        <div class="areas-header">
          <span>SLUG</span>
          <span>NAME</span>
          <span class="right">IDX</span>
          <span>SOURCE</span>
          <span></span>
        </div>
        {#each data.areas as area (area.id)}
          <div class="areas-row">
            <span class="row-slug">{area.slug}</span>
            <span class="row-name">{area.name}</span>
            <span class="row-idx right">{area.gameIndex ?? '—'}</span>
            <span class="src-badge src-{area.source}">{area.source}</span>
            {#if area.source === 'custom'}
              <form action="?/deleteArea" method="post" use:enhance>
                <input type="hidden" name="id" value={area.id}>
                <button type="submit" class="delete-area-btn">DELETE</button>
              </form>
            {:else}
              <span></span>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty">No areas yet.</p>
    {/if}
  </section>

  <!-- Encounters section -->
  <section class="enc-section">
    <div class="section-header">
      <h3 class="section-title">ENCOUNTERS ({data.encounters.length})</h3>
      <button type="button" class="new-btn" onclick={() => showCreateEncounter = !showCreateEncounter}>
        {showCreateEncounter ? '× CANCEL' : '+ ADD ENCOUNTER'}
      </button>
    </div>

    {#if showCreateEncounter}
      <form action="?/createEncounter" method="post" use:enhance class="enc-create-form">
        <div class="field">
          <label for="enc-area">AREA</label>
          <select id="enc-area" name="locationAreaId" required>
            {#each data.areas as a}
              <option value={a.id}>{a.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="enc-pkmn">POKÉMON</label>
          <select id="enc-pkmn" name="pokemonId" required>
            {#each data.allPokemon as p}
              <option value={p.id}>{p.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="enc-ver">VERSION</label>
          <select id="enc-ver" name="versionId" required>
            {#each data.allVersions as v}
              <option value={v.id}>{v.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="enc-method">METHOD</label>
          <select id="enc-method" name="encounterMethodId" required>
            {#each data.allMethods as m}
              <option value={m.id}>{m.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="enc-minlv">MIN LEVEL</label>
          <input id="enc-minlv" name="minLevel" type="number" min="1" max="100" required>
        </div>
        <div class="field">
          <label for="enc-maxlv">MAX LEVEL</label>
          <input id="enc-maxlv" name="maxLevel" type="number" min="1" max="100" required>
        </div>
        <div class="field">
          <label for="enc-chance">CHANCE %</label>
          <input id="enc-chance" name="chance" type="number" min="1" max="100" required>
        </div>
        <div class="field">
          <label for="enc-slot">SLOT</label>
          <input id="enc-slot" name="slot" type="number" min="0">
        </div>
        <div class="field">
          <label for="enc-cond">CONDITION</label>
          <select id="enc-cond" name="conditionValueId">
            <option value="">—</option>
            {#each data.allConditionValues as c}
              <option value={c.id}>{c.name}</option>
            {/each}
          </select>
        </div>
        <div class="form-actions">
          <button type="submit" class="save-btn">ADD</button>
          {#if form?.encError}
            <span class="form-error">{form.encError}</span>
          {/if}
        </div>
      </form>
    {/if}

    {#if data.encounters.length > 0}
      <div class="enc-table">
        <div class="enc-header">
          <span>VERSION</span>
          <span>AREA</span>
          <span>POKÉMON</span>
          <span>METHOD</span>
          <span class="center">LVL</span>
          <span class="center">CHANCE</span>
          <span>CONDITION</span>
          <span>SOURCE</span>
          <span></span>
        </div>
        {#each data.encounters as enc (enc.id)}
          <div class="enc-row">
            <span class="row-mono">{enc.versionName}</span>
            <span class="row-mono">{enc.areaName}</span>
            <span class="row-name">{enc.pokemonName}</span>
            <span class="row-mono">{enc.methodName}</span>
            <span class="row-mono center">{enc.minLevel}–{enc.maxLevel}</span>
            <span class="row-mono center">{enc.chance}%</span>
            <span class="row-mono">{enc.conditionValueName ?? '—'}</span>
            <span class="src-badge src-{enc.source}">{enc.source}</span>
            <form action="?/deleteEncounter" method="post" use:enhance>
              <input type="hidden" name="id" value={enc.id}>
              <button type="submit" class="delete-enc-btn">DELETE</button>
            </form>
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty">No encounters yet.</p>
    {/if}
  </section>

  {#if data.location.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn"
        onclick={(e) => { if (!confirm('Delete this location and all its areas?')) e.preventDefault(); }}>
        DELETE THIS LOCATION
      </button>
      <span class="delete-hint">Only custom locations can be deleted.</span>
      {#if form?.deleteError}
        <span class="form-error">{form.deleteError}</span>
      {/if}
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
  .page-header h2 { color: var(--theme-text-primary); margin: 0; }
  .meta { display: flex; gap: var(--spacing-3); font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
  .wiki-link { color: var(--theme-text-accent); text-decoration: none; }
  .wiki-link:hover { text-decoration: underline; }

  .src-badge { font-family: var(--font-display); font-size: 8px; padding: 2px 6px; border: 2px solid var(--theme-border-primary); }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input, .field select {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field input:focus, .field select:focus { outline: none; border-color: var(--theme-text-accent); }

  .form-actions { grid-column: 1 / -1; display: flex; align-items: center; gap: var(--spacing-3); }
  .save-btn {
    font-family: var(--font-display); font-size: 11px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: 2px solid var(--theme-text-accent); color: var(--theme-text-accent);
    cursor: pointer;
  }
  .save-btn:hover { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }
  .form-error { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-error); }
  .save-ok { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-accent); }

  /* Areas section */
  .areas-section { display: flex; flex-direction: column; gap: var(--spacing-3); }

  .section-header { display: flex; justify-content: space-between; align-items: center; }
  .section-title {
    font-family: var(--font-display); font-size: 11px; color: var(--theme-text-secondary);
    margin: 0; padding-bottom: var(--spacing-2); border-bottom: 2px solid var(--theme-border-primary);
    flex: 1;
  }

  .new-btn {
    font-family: var(--font-display); font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none; border: 2px solid var(--theme-border-primary); color: var(--theme-text-primary);
    cursor: pointer;
  }
  .new-btn:hover { border-color: var(--theme-text-accent); color: var(--theme-text-accent); }

  .area-create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .areas-table {
    display: flex; flex-direction: column;
    border: 3px solid var(--theme-border-primary);
  }

  .areas-header,
  .areas-row {
    display: grid;
    grid-template-columns: 1fr 1fr 60px 80px 70px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
  }

  .areas-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
  }

  .areas-row { border-bottom: 1px solid var(--theme-border-primary); }
  .areas-row:last-child { border-bottom: none; }
  .areas-row:nth-child(even) { background-color: var(--theme-bg-primary); }
  .areas-row:hover { background-color: var(--theme-bg-hover); }

  .row-slug { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }
  .row-name { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-primary); }
  .row-idx { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .right { text-align: right; }

  .empty { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); margin: 0; }

  .delete-area-btn {
    font-family: var(--font-display); font-size: 8px;
    padding: 2px 8px;
    background: none; border: 2px solid var(--color-text-error); color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-area-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }

  /* Encounters section */
  .enc-section { display: flex; flex-direction: column; gap: var(--spacing-3); }

  .enc-create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .enc-table {
    display: flex; flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow-x: auto;
    overflow-y: hidden;
  }

  .enc-header,
  .enc-row {
    display: grid;
    grid-template-columns: 90px 1fr 1fr 110px 70px 70px 110px 80px 70px;
    gap: var(--spacing-2);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
    min-width: 860px;
  }

  .enc-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
  }

  .enc-row { border-bottom: 1px solid var(--theme-border-primary); }
  .enc-row:last-child { border-bottom: none; }
  .enc-row:nth-child(even) { background-color: var(--theme-bg-primary); }
  .enc-row:hover { background-color: var(--theme-bg-hover); }

  .row-mono { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }
  .center { text-align: center; }

  .delete-enc-btn {
    font-family: var(--font-display); font-size: 8px;
    padding: 2px 8px;
    background: none; border: 2px solid var(--color-text-error); color: var(--color-text-error);
    cursor: pointer; white-space: nowrap;
  }
  .delete-enc-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }

  .delete-form { display: flex; align-items: center; gap: var(--spacing-3); flex-wrap: wrap; }
  .delete-btn {
    font-family: var(--font-display); font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: 2px solid var(--color-text-error); color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }
  .delete-hint { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
</style>
