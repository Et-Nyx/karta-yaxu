<script lang="ts">
  import { enhance } from '$app/forms';

  let { data, form } = $props();

  let saved = $state(false);
  let eggGroupsSaved = $state(false);

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

  function handleEggGroupsSave() {
    eggGroupsSaved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        eggGroupsSaved = true;
        setTimeout(() => eggGroupsSaved = false, 2500);
      }
    };
  }

  let s = $derived(form?.values ?? data.species);
</script>

<svelte:head>
  <title>Edit {data.species.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <a href="/editor/species" class="back">← BACK</a>
      <h2>EDIT SPECIES: {data.species.name.toUpperCase()}</h2>
      <span class="src-badge src-{data.species.source}">{data.species.source}</span>
    </div>
    <div class="meta">
      <span>ID: {data.species.id}</span>
      {#if data.species.pokeapiId}<span>PokéAPI: {data.species.pokeapiId}</span>{/if}
      <a href="/wiki/pokemon/{data.species.slug}" class="wiki-link">VIEW IN WIKI →</a>
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="80" required value={s.name ?? ''}>
    </div>
    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required value={s.slug ?? ''}>
    </div>
    <div class="field">
      <label for="dex">DEX #</label>
      <input id="dex" name="dexNumber" type="number" min="0" max="9999" required value={s.dexNumber ?? 0}>
    </div>
    <div class="field">
      <label for="genus">GENUS</label>
      <input id="genus" name="genus" type="text" placeholder="Mouse Pokémon" value={s.genus ?? ''}>
    </div>
    <div class="field">
      <label for="gen">GENERATION</label>
      <select id="gen" name="generationId">
        <option value="">— none —</option>
        {#each data.generations as g}
          <option value={g.id} selected={(s.generationId ?? null) === g.id}>{g.name}</option>
        {/each}
      </select>
    </div>

    <fieldset class="flags-group">
      <legend>FLAGS</legend>
      <div class="flag-row">
        <label><input name="isBaby" type="checkbox" value="true" checked={s.isBaby}> BABY</label>
        <label><input name="isLegendary" type="checkbox" value="true" checked={s.isLegendary}> LEGENDARY</label>
        <label><input name="isMythical" type="checkbox" value="true" checked={s.isMythical}> MYTHICAL</label>
      </div>
    </fieldset>

    <div class="field">
      <label for="habitat">HABITAT</label>
      <input id="habitat" name="habitat" type="text" placeholder="forest / cave / urban / ..." value={s.habitat ?? ''}>
    </div>
    <div class="field">
      <label for="color">COLOR</label>
      <input id="color" name="color" type="text" placeholder="yellow / red / ..." value={s.color ?? ''}>
    </div>
    <div class="field">
      <label for="shape">SHAPE</label>
      <input id="shape" name="shape" type="text" placeholder="quadruped / bipedal / ..." value={s.shape ?? ''}>
    </div>
    <div class="field">
      <label for="growth">GROWTH RATE</label>
      <input id="growth" name="growthRate" type="text" placeholder="medium-slow / fast / ..." value={s.growthRate ?? ''}>
    </div>

    <div class="field">
      <label for="happy">BASE HAPPINESS</label>
      <input id="happy" name="baseHappiness" type="number" value={s.baseHappiness ?? ''}>
    </div>
    <div class="field">
      <label for="capture">CAPTURE RATE</label>
      <input id="capture" name="captureRate" type="number" min="0" max="255" value={s.captureRate ?? ''}>
    </div>
    <div class="field">
      <label for="gender">GENDER RATE</label>
      <input id="gender" name="genderRate" type="number" min="-1" max="8" value={s.genderRate ?? ''}>
    </div>
    <div class="field">
      <label for="hatch">HATCH COUNTER</label>
      <input id="hatch" name="hatchCounter" type="number" value={s.hatchCounter ?? ''}>
    </div>

    <div class="field">
      <label for="order">ORDER</label>
      <input id="order" name="order" type="number" value={s.order ?? ''}>
    </div>
    <div class="field">
      <label for="dexorder">DEX ORDER</label>
      <input id="dexorder" name="dexOrder" type="number" value={s.dexOrder ?? ''}>
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

  <section class="sub-section">
    <h3 class="section-title">EGG GROUPS</h3>
    <form action="?/updateEggGroups" method="post" use:enhance={handleEggGroupsSave} class="sub-form">
      <div class="field full">
        <label for="egg-groups">EGG GROUPS (hold Ctrl/Cmd to select multiple)</label>
        <select id="egg-groups" name="eggGroupIds" multiple size="6">
          {#each data.allEggGroups as g}
            <option value={g.id} selected={data.currentEggGroups.some(c => c.eggGroupId === g.id)}>{g.name}</option>
          {/each}
        </select>
      </div>
      <div class="form-actions">
        <button type="submit" class="save-btn">SAVE EGG GROUPS</button>
        {#if form?.eggGroupsError}<span class="form-error">{form.eggGroupsError}</span>{/if}
        {#if eggGroupsSaved}<span class="save-ok">✓ SAVED</span>{/if}
      </div>
    </form>
  </section>

  <section class="linked-pokemon">
    <h3 class="section-title">LINKED POKEMON ({data.pokemon.length})</h3>
    {#if data.pokemon.length === 0}
      <div class="empty-state">
        <p>No pokemon rows linked to this species.</p>
      </div>
    {:else}
      <div class="pokemon-list">
        {#each data.pokemon as p (p.id)}
          <a href="/editor/pokemon/{p.slug}" class="pokemon-row card-interactive">
            <span class="pokemon-name">
              {p.name}
              {#if p.isDefault}<span class="default-flag">[BASE FORM]</span>{/if}
            </span>
            <span class="pokemon-slug">{p.slug}</span>
            <span class="src-badge src-{p.source}">{p.source}</span>
          </a>
        {/each}
      </div>
    {/if}
  </section>

  {#if data.species.source === 'edited' && data.species.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import this species from PokeAPI? Genus, dex number, flags, evolution chain link, flavor texts, and egg groups will be restored to canonical PokeAPI data.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Reverts to canonical PokeAPI data; source flips back to 'pokeapi'.</span>
    </form>
  {/if}

  {#if data.species.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS SPECIES</button>
      <span class="delete-hint">
        Apenas species custom podem ser deletadas. Todos os pokemon linkados, seus types, abilities, learnset e forms são removidos junto.
      </span>
    </form>
  {/if}
</div>

<style>
  .edit-page { display: flex; flex-direction: column; gap: var(--spacing-5); }
  .page-header { display: flex; justify-content: space-between; align-items: flex-end; gap: var(--spacing-3); flex-wrap: wrap; }
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

  .edit-form, .sub-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }
  .sub-section { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field.full { grid-column: 1 / -1; }
  .field select[multiple] { min-height: 120px; }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input, .field select {
    font-family: var(--font-mono); font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field input:focus, .field select:focus { outline: none; border-color: var(--theme-text-accent); }

  .flags-group {
    grid-column: 1 / -1;
    border: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3);
  }
  .flags-group legend {
    font-family: var(--font-display); font-size: 9px;
    color: var(--theme-text-muted); padding: 0 var(--spacing-2);
  }
  .flag-row { display: flex; gap: var(--spacing-4); flex-wrap: wrap; }
  .flag-row label {
    display: flex; align-items: center; gap: var(--spacing-2);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
    cursor: pointer;
  }
  .flag-row input[type="checkbox"] { width: auto; }

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

  .linked-pokemon { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .section-title {
    font-family: var(--font-display); font-size: 11px; color: var(--theme-text-secondary);
    margin: 0; padding-bottom: var(--spacing-2); border-bottom: 2px solid var(--theme-border-primary);
  }
  .pokemon-list { display: flex; flex-direction: column; gap: var(--spacing-2); }
  .pokemon-row {
    display: grid;
    grid-template-columns: 1fr 1fr 80px;
    gap: var(--spacing-3);
    align-items: center;
    padding: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    text-decoration: none;
    color: var(--theme-text-primary);
  }
  .pokemon-name { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-primary); }
  .default-flag { font-size: 9px; color: var(--theme-text-accent); margin-left: var(--spacing-2); }
  .pokemon-slug { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    text-align: center;
  }
  .empty-state p { font-family: var(--font-mono); font-size: 12px; color: var(--theme-text-muted); margin: 0; }

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
