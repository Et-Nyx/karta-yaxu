<script lang="ts">
  import { enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';

  let { data, form } = $props();

  let saved = $state(false);
  let typesSaved = $state(false);
  let abilitiesSaved = $state(false);

  let moveQuery = $state('');
  let selectedMoveId = $derived(
    data.allMoves.find(m => m.name.toLowerCase() === moveQuery.toLowerCase())?.id ?? null
  );

  let selectedVgSlug = $derived(
    data.vgCounts.find(v => v.id === data.selectedVgId)?.slug ?? ''
  );

  function onVgChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newSlug = target.value;
    const url = new URL($page.url);
    if (newSlug) url.searchParams.set('vg', newSlug);
    else url.searchParams.delete('vg');
    goto(url.toString(), { keepFocus: true, noScroll: true, invalidateAll: true });
  }

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

  function handleTypesSave() {
    typesSaved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        typesSaved = true;
        setTimeout(() => typesSaved = false, 2500);
      }
    };
  }

  function handleAbilitiesSave() {
    abilitiesSaved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        abilitiesSaved = true;
        setTimeout(() => abilitiesSaved = false, 2500);
      }
    };
  }

  let p = $derived(form?.values ?? data.pokemon);

  let currentSlot1Type = $derived(data.types.find(t => t.slot === 1)?.typeId ?? null);
  let currentSlot2Type = $derived(data.types.find(t => t.slot === 2)?.typeId ?? null);

  let slot1Ability = $derived(data.abilities.find(a => a.slot === 1 && !a.isHidden)?.abilityId ?? null);
  let slot2Ability = $derived(data.abilities.find(a => a.slot === 2 && !a.isHidden)?.abilityId ?? null);
  let hiddenAbility = $derived(data.abilities.find(a => a.isHidden)?.abilityId ?? null);
</script>

<svelte:head>
  <title>Edit {data.pokemon.name} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <a href="/editor/pokemon" class="back">← BACK</a>
      <h2>EDIT POKÉMON: {data.pokemon.name.toUpperCase()}</h2>
      <span class="src-badge src-{data.pokemon.source}">{data.pokemon.source}</span>
    </div>
    <div class="meta">
      <span>ID: {data.pokemon.id}</span>
      {#if data.pokemon.pokeapiId}<span>PokéAPI: {data.pokemon.pokeapiId}</span>{/if}
      <a href="/wiki/pokemon/{data.pokemon.slug}" class="wiki-link">VIEW IN WIKI →</a>
    </div>
  </header>

  {#if data.pokemon.spriteDefault}
    <div class="sprite-preview">
      <img src={data.pokemon.spriteDefault} alt={data.pokemon.name}>
    </div>
  {/if}

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="name">NAME</label>
      <input id="name" name="name" type="text" maxlength="80" required value={p.name ?? ''}>
    </div>
    <div class="field">
      <label for="slug">SLUG</label>
      <input id="slug" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required value={p.slug ?? ''}>
    </div>
    <div class="field">
      <label for="species">SPECIES</label>
      <select id="species" name="speciesId" required>
        {#each data.species as s}
          <option value={s.id} selected={(p.speciesId ?? null) === s.id}>#{s.dexNumber} {s.name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="height">HEIGHT (dm)</label>
      <input id="height" name="height" type="number" min="0" required value={p.height ?? 0}>
    </div>
    <div class="field">
      <label for="weight">WEIGHT (hg)</label>
      <input id="weight" name="weight" type="number" min="0" required value={p.weight ?? 0}>
    </div>
    <div class="field field-check">
      <label for="default">
        <input id="default" name="isDefault" type="checkbox" value="true" checked={p.isDefault}>
        IS DEFAULT
      </label>
    </div>
    <div class="field">
      <label for="order">ORDER</label>
      <input id="order" name="order" type="number" value={p.order ?? ''}>
    </div>

    <fieldset class="stats-group">
      <legend>STATS</legend>
      <div class="stat-row">
        {#each ['hp','atk','def','spa','spd','spe'] as stat}
          <div class="field stat-field">
            <label for={stat}>{stat.toUpperCase()}</label>
            <input id={stat} name={stat} type="number" min="0" max="255" required value={(p as any)[stat] ?? 0}>
          </div>
        {/each}
      </div>
    </fieldset>

    <div class="field full">
      <label for="spriteDefault">SPRITE DEFAULT</label>
      <input id="spriteDefault" name="spriteDefault" type="text" value={p.spriteDefault ?? ''}>
    </div>
    <div class="field full">
      <label for="spriteShiny">SPRITE SHINY</label>
      <input id="spriteShiny" name="spriteShiny" type="text" value={p.spriteShiny ?? ''}>
    </div>
    <div class="field full">
      <label for="spriteArtwork">SPRITE ARTWORK</label>
      <input id="spriteArtwork" name="spriteArtwork" type="text" value={p.spriteArtwork ?? ''}>
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

  <!-- Types sub-form -->
  <section class="sub-section">
    <h3 class="section-title">TYPES</h3>
    <form action="?/updateTypes" method="post" use:enhance={handleTypesSave} class="sub-form">
      <div class="field">
        <label for="t1">SLOT 1 (PRIMARY)</label>
        <select id="t1" name="slot1TypeId" required>
          {#each data.allTypes as t}
            <option value={t.id} selected={currentSlot1Type === t.id}>{t.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for="t2">SLOT 2 (SECONDARY)</label>
        <select id="t2" name="slot2TypeId">
          <option value="">— none —</option>
          {#each data.allTypes as t}
            <option value={t.id} selected={currentSlot2Type === t.id}>{t.name}</option>
          {/each}
        </select>
      </div>
      <div class="form-actions">
        <button type="submit" class="save-btn">SAVE TYPES</button>
        {#if form?.typesError}
          <span class="form-error">{form.typesError}</span>
        {/if}
        {#if typesSaved}
          <span class="save-ok">✓ SAVED</span>
        {/if}
      </div>
    </form>
  </section>

  <!-- Abilities sub-form -->
  <section class="sub-section">
    <h3 class="section-title">ABILITIES</h3>
    <form action="?/updateAbilities" method="post" use:enhance={handleAbilitiesSave} class="sub-form">
      <div class="field">
        <label for="a1">SLOT 1</label>
        <select id="a1" name="slot1AbilityId" required>
          {#each data.allAbilities as a}
            <option value={a.id} selected={slot1Ability === a.id}>{a.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for="a2">SLOT 2</label>
        <select id="a2" name="slot2AbilityId">
          <option value="">— none —</option>
          {#each data.allAbilities as a}
            <option value={a.id} selected={slot2Ability === a.id}>{a.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label for="ah">HIDDEN</label>
        <select id="ah" name="hiddenAbilityId">
          <option value="">— none —</option>
          {#each data.allAbilities as a}
            <option value={a.id} selected={hiddenAbility === a.id}>{a.name}</option>
          {/each}
        </select>
      </div>
      <div class="form-actions">
        <button type="submit" class="save-btn">SAVE ABILITIES</button>
        {#if form?.abilitiesError}
          <span class="form-error">{form.abilitiesError}</span>
        {/if}
        {#if abilitiesSaved}
          <span class="save-ok">✓ SAVED</span>
        {/if}
      </div>
    </form>
  </section>

  <!-- FORMS sub-section -->
  <section class="sub-section">
    <h3 class="section-title">FORMS ({data.forms.length})</h3>

    {#if data.forms.length === 0}
      <div class="empty-state">
        <p>No forms recorded for this Pokémon.</p>
        <p class="hint">Most Pokémon have exactly one default form. Use "+ NEW FORM" to add Gmax/Mega/Cosplay variants.</p>
      </div>
    {:else}
      <div class="forms-list">
        {#each data.forms as f (f.id)}
          <form action="?/updateForm" method="post" use:enhance class="form-row">
            <input type="hidden" name="formId" value={f.id}>
            <div class="form-row-head">
              <span class="form-name">
                {f.formName ? `“${f.formName}”` : '— default —'}
                {#if f.isDefault}<span class="default-flag">[DEFAULT]</span>{/if}
              </span>
              <span class="src-badge src-{f.source}">{f.source}</span>
            </div>
            <div class="form-grid">
              <div class="field">
                <label for="fn-{f.id}">FORM NAME</label>
                <input id="fn-{f.id}" name="formName" type="text" value={f.formName ?? ''} placeholder="gmax / mega-x / alola / ...">
              </div>
              <div class="field">
                <label for="fo-{f.id}">ORDER</label>
                <input id="fo-{f.id}" name="order" type="number" value={f.order ?? ''}>
              </div>
              <div class="field field-check">
                <label><input name="isDefault" type="checkbox" value="true" checked={f.isDefault}> DEFAULT</label>
              </div>
              <div class="field field-check">
                <label><input name="isBattleOnly" type="checkbox" value="true" checked={f.isBattleOnly}> BATTLE ONLY</label>
              </div>
              <div class="field field-check">
                <label><input name="isMega" type="checkbox" value="true" checked={f.isMega}> MEGA</label>
              </div>
              <div class="field full">
                <label for="fsd-{f.id}">SPRITE DEFAULT</label>
                <input id="fsd-{f.id}" name="spriteDefault" type="text" value={f.spriteDefault ?? ''}>
              </div>
              <div class="field full">
                <label for="fss-{f.id}">SPRITE SHINY</label>
                <input id="fss-{f.id}" name="spriteShiny" type="text" value={f.spriteShiny ?? ''}>
              </div>
              <div class="field full">
                <label for="fsa-{f.id}">SPRITE ARTWORK</label>
                <input id="fsa-{f.id}" name="spriteArtwork" type="text" value={f.spriteArtwork ?? ''}>
              </div>
            </div>
            <div class="form-row-actions">
              <button type="submit" class="save-btn">SAVE FORM</button>
              {#if f.source === 'custom'}
                <button type="submit" formaction="?/deleteForm" class="delete-btn-sm" onclick={(e) => { if (!confirm('Delete this form?')) e.preventDefault(); }}>DELETE</button>
              {/if}
            </div>
          </form>
        {/each}
      </div>
    {/if}

    <details class="add-form-toggle">
      <summary>+ NEW FORM</summary>
      <form action="?/addForm" method="post" use:enhance class="form-row add-form">
        <div class="form-grid">
          <div class="field">
            <label for="new-fn">FORM NAME</label>
            <input id="new-fn" name="formName" type="text" placeholder="gmax / mega / ...">
          </div>
          <div class="field">
            <label for="new-fo">ORDER</label>
            <input id="new-fo" name="order" type="number">
          </div>
          <div class="field field-check">
            <label><input name="isDefault" type="checkbox" value="true"> DEFAULT</label>
          </div>
          <div class="field field-check">
            <label><input name="isBattleOnly" type="checkbox" value="true"> BATTLE ONLY</label>
          </div>
          <div class="field field-check">
            <label><input name="isMega" type="checkbox" value="true"> MEGA</label>
          </div>
          <div class="field full">
            <label for="new-fsd">SPRITE DEFAULT</label>
            <input id="new-fsd" name="spriteDefault" type="text">
          </div>
          <div class="field full">
            <label for="new-fss">SPRITE SHINY</label>
            <input id="new-fss" name="spriteShiny" type="text">
          </div>
          <div class="field full">
            <label for="new-fsa">SPRITE ARTWORK</label>
            <input id="new-fsa" name="spriteArtwork" type="text">
          </div>
        </div>
        <div class="form-row-actions">
          <button type="submit" class="save-btn">CREATE FORM</button>
        </div>
      </form>
    </details>

    {#if form?.formError}
      <p class="form-error">{form.formError}</p>
    {/if}
  </section>

  <!-- EVOLUTION sub-section -->
  <section class="sub-section">
    <h3 class="section-title">EVOLUTION</h3>

    <!-- Predecessor panel -->
    <div class="evo-panel">
      <h4 class="evo-panel-title">PREDECESSOR</h4>
      {#if data.predecessor}
        <div class="evo-row">
          <a href="/editor/pokemon/{data.predecessor.slug}" class="evo-link">← {data.predecessor.name}</a>
          {#if data.predecessorConditions}
            <span class="evo-conditions">
              {data.predecessorConditions.trigger}
              {#if data.predecessorConditions.minLevel}· lvl {data.predecessorConditions.minLevel}{/if}
              {#if data.predecessorConditions.itemId}· item {data.allItems.find(i => i.id === data.predecessorConditions?.itemId)?.name ?? `#${data.predecessorConditions.itemId}`}{/if}
              {#if data.predecessorConditions.minHappiness}· happiness {data.predecessorConditions.minHappiness}{/if}
              {#if data.predecessorConditions.timeOfDay}· {data.predecessorConditions.timeOfDay}{/if}
              {#if data.predecessorConditions.location}· at {data.predecessorConditions.location}{/if}
            </span>
          {/if}
        </div>
        <details class="evo-edit-toggle">
          <summary>EDIT CONDITIONS</summary>
          <form action="?/updateConditions" method="post" use:enhance class="sub-form">
            <div class="field">
              <label for="cond-trigger">TRIGGER</label>
              <select id="cond-trigger" name="trigger" required>
                {#each ['level-up', 'use-item', 'use-move', 'trade', 'three-critical-hits', 'shed', 'spin'] as t}
                  <option value={t} selected={data.predecessorConditions?.trigger === t}>{t}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="cond-level">MIN LEVEL</label>
              <input id="cond-level" name="minLevel" type="number" min="0" max="100" value={data.predecessorConditions?.minLevel ?? ''}>
            </div>
            <div class="field">
              <label for="cond-item">ITEM (use-item)</label>
              <select id="cond-item" name="itemId">
                <option value="">— none —</option>
                {#each data.allItems as i}
                  <option value={i.id} selected={(data.predecessorConditions?.itemId ?? null) === i.id}>{i.name}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="cond-held">HELD ITEM</label>
              <select id="cond-held" name="heldItemId">
                <option value="">— none —</option>
                {#each data.allItems as i}
                  <option value={i.id} selected={(data.predecessorConditions?.heldItemId ?? null) === i.id}>{i.name}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="cond-move">KNOWN MOVE</label>
              <select id="cond-move" name="knownMoveId">
                <option value="">— none —</option>
                {#each data.allMoves as m}
                  <option value={m.id} selected={(data.predecessorConditions?.knownMoveId ?? null) === m.id}>{m.name}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="cond-movetype">KNOWN MOVE TYPE</label>
              <select id="cond-movetype" name="knownMoveTypeId">
                <option value="">— none —</option>
                {#each data.allTypes as t}
                  <option value={t.id} selected={(data.predecessorConditions?.knownMoveTypeId ?? null) === t.id}>{t.name}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="cond-happy">MIN HAPPINESS</label>
              <input id="cond-happy" name="minHappiness" type="number" min="0" max="255" value={data.predecessorConditions?.minHappiness ?? ''}>
            </div>
            <div class="field">
              <label for="cond-time">TIME OF DAY</label>
              <select id="cond-time" name="timeOfDay">
                <option value="">— any —</option>
                {#each ['day', 'night', 'dusk'] as t}
                  <option value={t} selected={data.predecessorConditions?.timeOfDay === t}>{t}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="cond-gender">GENDER</label>
              <select id="cond-gender" name="gender">
                <option value="">— any —</option>
                <option value="1" selected={data.predecessorConditions?.gender === 1}>female</option>
                <option value="2" selected={data.predecessorConditions?.gender === 2}>male</option>
              </select>
            </div>
            <div class="field">
              <label for="cond-loc">LOCATION</label>
              <input id="cond-loc" name="location" type="text" value={data.predecessorConditions?.location ?? ''}>
            </div>
            <div class="form-actions">
              <button type="submit" class="save-btn">SAVE CONDITIONS</button>
            </div>
          </form>
        </details>
        <form action="?/clearPredecessor" method="post" use:enhance class="delete-form">
          <button type="submit" class="delete-btn-sm" onclick={(e) => { if (!confirm('Remove predecessor and conditions?')) e.preventDefault(); }}>CLEAR PREDECESSOR</button>
        </form>
      {:else}
        <p class="evo-empty">Root of an evolution chain (no predecessor).</p>
        <details class="evo-edit-toggle">
          <summary>+ SET PREDECESSOR</summary>
          <form action="?/setPredecessor" method="post" use:enhance class="sub-form">
            <div class="field full">
              <label for="set-parent">PARENT POKÉMON</label>
              <select id="set-parent" name="parentPokemonId" required>
                <option value="">— pick a parent —</option>
                {#each data.allPokemon as p}
                  <option value={p.id}>{p.name} ({p.slug})</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="set-trigger">TRIGGER</label>
              <select id="set-trigger" name="trigger" required>
                {#each ['level-up', 'use-item', 'use-move', 'trade', 'three-critical-hits', 'shed', 'spin'] as t}
                  <option value={t}>{t}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="set-level">MIN LEVEL</label>
              <input id="set-level" name="minLevel" type="number" min="0" max="100">
            </div>
            <div class="field">
              <label for="set-item">ITEM</label>
              <select id="set-item" name="itemId">
                <option value="">— none —</option>
                {#each data.allItems as i}<option value={i.id}>{i.name}</option>{/each}
              </select>
            </div>
            <div class="field">
              <label for="set-happy">MIN HAPPINESS</label>
              <input id="set-happy" name="minHappiness" type="number" min="0" max="255">
            </div>
            <div class="field">
              <label for="set-time">TIME OF DAY</label>
              <select id="set-time" name="timeOfDay">
                <option value="">— any —</option>
                {#each ['day', 'night', 'dusk'] as t}<option value={t}>{t}</option>{/each}
              </select>
            </div>
            <div class="form-actions">
              <button type="submit" class="save-btn">SET PREDECESSOR</button>
            </div>
          </form>
        </details>
      {/if}
    </div>

    <!-- Successors panel -->
    <div class="evo-panel">
      <h4 class="evo-panel-title">SUCCESSORS ({data.successors.length})</h4>
      {#if data.successors.length === 0}
        <p class="evo-empty">No successors recorded. To add one, edit the target pokemon and set this one as its predecessor.</p>
      {:else}
        <div class="evo-successors">
          {#each data.successors as s (s.id)}
            <a href="/editor/pokemon/{s.slug}" class="evo-link evo-successor card-interactive">
              → {s.name}
              {#if s.trigger}
                <span class="evo-conditions">
                  {s.trigger}
                  {#if s.minLevel}· lvl {s.minLevel}{/if}
                  {#if s.itemId}· item {data.allItems.find(i => i.id === s.itemId)?.name ?? `#${s.itemId}`}{/if}
                </span>
              {/if}
            </a>
          {/each}
        </div>
      {/if}
    </div>

    {#if form?.evoError}
      <p class="form-error">{form.evoError}</p>
    {/if}
  </section>

  <!-- LEARNSET sub-section -->
  <section class="sub-section">
    <h3 class="section-title">LEARNSET</h3>

    <div class="learnset-controls">
      <div class="field">
        <label for="vg-select">VERSION GROUP</label>
        <select id="vg-select" onchange={onVgChange}>
          {#each data.vgCounts as v}
            <option value={v.slug} selected={v.id === data.selectedVgId}>{v.slug} ({v.count})</option>
          {/each}
        </select>
      </div>

      {#if data.selectedVgId != null && data.learnset.length > 0}
        <form action="?/clearLearnsetForVg" method="post" use:enhance>
          <input type="hidden" name="versionGroupId" value={data.selectedVgId}>
          <button type="submit" class="delete-btn-sm" onclick={(e) => { if (!confirm(`Clear all ${data.learnset.length} entries in this version group?`)) e.preventDefault(); }}>
            CLEAR THIS VG
          </button>
        </form>
      {/if}

      <details class="copy-toggle">
        <summary>COPY FROM ANOTHER VG</summary>
        <form action="?/copyLearnsetFromVg" method="post" use:enhance class="copy-form">
          <input type="hidden" name="targetVgId" value={data.selectedVgId}>
          <select name="sourceVgId" required>
            <option value="">— pick source vg —</option>
            {#each data.vgCounts.filter(v => v.id !== data.selectedVgId && v.count > 0) as v}
              <option value={v.id}>{v.slug} ({v.count})</option>
            {/each}
          </select>
          <button type="submit" class="save-btn">COPY</button>
        </form>
      </details>
    </div>

    {#if data.selectedVgId == null}
      <p class="evo-empty">No version groups with learnset data for this Pokémon. Add an entry below to start one.</p>
    {/if}

    {#if data.learnset.length > 0}
      <div class="learnset-table">
        <div class="learnset-head">
          <span>METHOD</span>
          <span>MOVE</span>
          <span>LEVEL</span>
          <span></span>
        </div>
        {#each data.learnset as entry, i (`${entry.moveId}-${entry.learnMethodId}-${entry.level}`)}
          <form action="?/removeLearnsetEntry" method="post" use:enhance class="learnset-row">
            <input type="hidden" name="moveId" value={entry.moveId}>
            <input type="hidden" name="learnMethodId" value={entry.learnMethodId}>
            <input type="hidden" name="level" value={entry.level ?? 0}>
            <input type="hidden" name="versionGroupId" value={data.selectedVgId}>
            <span class="learnset-method">{entry.learnMethodName}</span>
            <a href="/wiki/moves/{entry.moveSlug}" class="learnset-move">{entry.moveName}</a>
            <span class="learnset-level">{entry.level ?? '—'}</span>
            <button type="submit" class="delete-btn-sm">REMOVE</button>
          </form>
        {/each}
      </div>
    {:else if data.selectedVgId != null}
      <p class="evo-empty">No entries in this version group yet. Use the form below to add one.</p>
    {/if}

    <details class="add-form-toggle">
      <summary>+ ADD MOVE TO LEARNSET</summary>
      <form action="?/addLearnsetEntry" method="post" use:enhance class="sub-form add-form">
        <input type="hidden" name="versionGroupId" value={data.selectedVgId}>
        <div class="field">
          <label for="add-move">MOVE</label>
          <input
            id="add-move"
            list="all-moves-list"
            bind:value={moveQuery}
            placeholder="type to search…"
            autocomplete="off"
            required
          >
          <datalist id="all-moves-list">
            {#each data.allMoves as m}
              <option value={m.name}></option>
            {/each}
          </datalist>
          <input type="hidden" name="moveId" value={selectedMoveId ?? ''}>
          {#if moveQuery && selectedMoveId == null}
            <span class="prefill-msg">no match (type a move name from the list)</span>
          {/if}
        </div>
        <div class="field">
          <label for="add-method">METHOD</label>
          <select id="add-method" name="learnMethodId" required>
            {#each data.allLearnMethods as m}
              <option value={m.id} selected={m.slug === 'level-up'}>{m.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="add-level">LEVEL</label>
          <input id="add-level" name="level" type="number" min="0" max="100" value="0">
        </div>
        <div class="form-actions">
          <button type="submit" class="save-btn">ADD ENTRY</button>
        </div>
      </form>
    </details>

    {#if form?.learnsetError}
      <p class="form-error">{form.learnsetError}</p>
    {/if}
  </section>

  {#if data.pokemon.source === 'edited' && data.pokemon.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import this Pokémon from PokeAPI? Stats, types, abilities, sprites, and learnset will be restored to canonical PokeAPI data. Form-specific evolution edges (linked-list) are NOT re-sourced.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Restores stats / types / abilities / learnset. Evolution pointer is preserved.</span>
    </form>
  {/if}

  {#if data.pokemon.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS POKÉMON</button>
      <span class="delete-hint">Apenas pokémon custom podem ser deletados. Sub-tabelas (types, abilities, forms) são removidas junto.</span>
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

  .sprite-preview {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    display: flex; justify-content: center;
  }
  .sprite-preview img {
    max-height: 96px;
    image-rendering: pixelated;
  }

  .edit-form, .sub-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field.full { grid-column: 1 / -1; }
  .field-check { flex-direction: row; align-items: center; }
  .field-check label {
    display: flex; align-items: center; gap: var(--spacing-2);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
    cursor: pointer;
  }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input, .field select {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field input[type="checkbox"] { width: auto; padding: 0; }
  .field input:focus, .field select:focus {
    outline: none; border-color: var(--theme-text-accent);
  }

  .stats-group {
    grid-column: 1 / -1;
    border: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3);
  }
  .stats-group legend {
    font-family: var(--font-display); font-size: 9px;
    color: var(--theme-text-muted); padding: 0 var(--spacing-2);
  }
  .stat-row { display: grid; grid-template-columns: repeat(6, 1fr); gap: var(--spacing-2); }
  .stat-field input { font-size: 11px; }

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

  .sub-section { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .section-title {
    font-family: var(--font-display); font-size: 11px; color: var(--theme-text-secondary);
    margin: 0; padding-bottom: var(--spacing-2); border-bottom: 2px solid var(--theme-border-primary);
  }

  /* Forms sub-section */
  .forms-list { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .form-row {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }
  .form-row-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .form-name {
    font-family: var(--font-display); font-size: 11px;
    color: var(--theme-text-primary);
  }
  .default-flag {
    font-size: 9px; color: var(--theme-text-accent); margin-left: var(--spacing-2);
  }
  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-2);
  }
  .form-row-actions {
    display: flex; align-items: center; gap: var(--spacing-3);
  }
  .delete-btn-sm {
    font-family: var(--font-display); font-size: 9px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none; border: 2px solid var(--color-text-error); color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-btn-sm:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }
  .add-form-toggle {
    background-color: var(--theme-bg-secondary);
    border: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3);
  }
  .add-form-toggle summary {
    font-family: var(--font-display); font-size: 10px;
    color: var(--theme-text-accent);
    cursor: pointer;
    list-style: none;
  }
  .add-form-toggle[open] summary { margin-bottom: var(--spacing-3); }
  .add-form { border: none; padding: 0; }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3);
    text-align: center;
  }
  .empty-state p {
    font-family: var(--font-mono); font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-1) 0;
  }
  .empty-state .hint { font-size: 10px; color: var(--theme-text-muted); }

  .src-badge {
    font-family: var(--font-display); font-size: 8px;
    padding: 2px 6px; border: 2px solid var(--theme-border-primary);
  }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  /* Evolution sub-section */
  .evo-panel {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }
  .evo-panel-title {
    font-family: var(--font-display); font-size: 9px;
    color: var(--theme-text-muted);
    margin: 0;
  }
  .evo-row { display: flex; align-items: center; gap: var(--spacing-3); flex-wrap: wrap; }
  .evo-link {
    font-family: var(--font-display); font-size: 11px;
    color: var(--theme-text-accent);
    text-decoration: none;
  }
  .evo-link:hover { text-decoration: underline; }
  .evo-conditions {
    font-family: var(--font-mono); font-size: 10px;
    color: var(--theme-text-muted);
  }
  .evo-empty {
    font-family: var(--font-mono); font-size: 11px;
    color: var(--theme-text-muted);
    margin: 0;
  }
  .evo-edit-toggle {
    border: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-2);
  }
  .evo-edit-toggle summary {
    font-family: var(--font-display); font-size: 9px;
    color: var(--theme-text-accent);
    cursor: pointer;
    list-style: none;
  }
  .evo-edit-toggle[open] summary { margin-bottom: var(--spacing-3); }
  .evo-successors {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }
  .evo-successor {
    display: flex; flex-direction: column; gap: var(--spacing-1);
    padding: var(--spacing-2) var(--spacing-3);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
  }

  /* Learnset sub-section */
  .learnset-controls {
    display: flex;
    gap: var(--spacing-3);
    align-items: end;
    flex-wrap: wrap;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
  }
  .learnset-controls .field { min-width: 220px; }
  .copy-toggle {
    border: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-2);
  }
  .copy-toggle summary {
    font-family: var(--font-display); font-size: 9px;
    color: var(--theme-text-accent);
    cursor: pointer;
    list-style: none;
  }
  .copy-form {
    display: flex; align-items: center; gap: var(--spacing-2);
    margin-top: var(--spacing-2);
  }
  .copy-form select {
    font-family: var(--font-mono); font-size: 11px;
    padding: var(--spacing-1) var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }

  .learnset-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
  }
  .learnset-head, .learnset-row {
    display: grid;
    grid-template-columns: 140px 1fr 60px 80px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
  }
  .learnset-head {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted);
  }
  .learnset-row { border-bottom: 1px solid var(--theme-border-primary); }
  .learnset-row:last-child { border-bottom: none; }
  .learnset-row:hover { background-color: var(--theme-bg-hover); }
  .learnset-method { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .learnset-move {
    font-family: var(--font-display); font-size: 11px;
    color: var(--theme-text-primary);
    text-decoration: none;
  }
  .learnset-move:hover { color: var(--theme-text-accent); text-decoration: underline; }
  .learnset-level { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }

  .delete-form { display: flex; align-items: center; gap: var(--spacing-3); }
  .delete-btn {
    font-family: var(--font-display); font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: 2px solid var(--color-text-error); color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }
  .delete-hint { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }

  .prefill-msg {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-warning);
    margin-top: var(--spacing-1);
  }
</style>
