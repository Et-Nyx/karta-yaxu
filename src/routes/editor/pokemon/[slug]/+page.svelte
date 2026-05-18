<script lang="ts">
  import { enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import { pageSource } from '$lib/stores/pageSource';

  let { data, form } = $props();

  $effect(() => { pageSource.set(data.pokemon.source); });

  // Save states
  let saved = $state(false);

  // --- Main update form state ---
  let nameVal = $state(data.pokemon.name);
  let slugVal = $state(data.pokemon.slug);
  let speciesIdVal = $state(data.pokemon.speciesId);
  let heightVal = $state(data.pokemon.height);
  let weightVal = $state(data.pokemon.weight);
  let isDefaultVal = $state(data.pokemon.isDefault);
  let orderVal = $state<number | ''>(data.pokemon.order ?? '');

  let spriteDefaultVal = $state(data.pokemon.spriteDefault ?? '');
  let spriteShinyVal = $state(data.pokemon.spriteShiny ?? '');
  let spriteArtworkVal = $state(data.pokemon.spriteArtwork ?? '');
  let isShiny = $state(false);
  let currentSpriteUrl = $derived(
    isShiny
      ? (data.pokemon.pokeapiId
          ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/shiny/${data.pokemon.pokeapiId}.png`
          : (spriteShinyVal || null))
      : (spriteArtworkVal || spriteDefaultVal || null)
  );

  let hpVal  = $state(data.pokemon.hp);
  let atkVal = $state(data.pokemon.atk);
  let defVal = $state(data.pokemon.def);
  let spaVal = $state(data.pokemon.spa);
  let spdVal = $state(data.pokemon.spd);
  let speVal = $state(data.pokemon.spe);
  let totalStats = $derived(hpVal + atkVal + defVal + spaVal + spdVal + speVal);

  // --- Reactive types for header TypeBadges ---
  let slot1TypeId = $state<number | null>(data.types.find(t => t.slot === 1)?.typeId ?? null);
  let slot2TypeId = $state<number | null>(data.types.find(t => t.slot === 2)?.typeId ?? null);
  let slot1Type = $derived(data.allTypes.find(t => t.id === slot1TypeId));
  let slot2Type = $derived(data.allTypes.find(t => t.id === slot2TypeId));

  // --- Abilities ---
  let slot1AbilityVal = $state<number | null>(data.abilities.find(a => a.slot === 1 && !a.isHidden)?.abilityId ?? null);
  let slot2AbilityVal = $state<number | null>(data.abilities.find(a => a.slot === 2 && !a.isHidden)?.abilityId ?? null);
  let hiddenAbilityVal = $state<number | null>(data.abilities.find(a => a.isHidden)?.abilityId ?? null);

  // --- Species lookup for header display ---
  let currentSpecies = $derived(data.species.find(s => s.id === speciesIdVal));

  // --- Learnset ---
  let moveQuery = $state('');
  let selectedMoveId = $derived(
    data.allMoves.find(m => m.name.toLowerCase() === moveQuery.toLowerCase())?.id ?? null
  );

  function onVgChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newSlug = target.value;
    const url = new URL($page.url);
    if (newSlug) url.searchParams.set('vg', newSlug);
    else url.searchParams.delete('vg');
    goto(url.toString(), { keepFocus: true, noScroll: true, invalidateAll: true });
  }

  // --- Form sub-tabs (species forms grouped by slug pattern) ---
  const REGIONAL_SUFFIXES = ['alola', 'galar', 'hisui', 'paldea'] as const;
  type SpeciesForm = typeof data.speciesForms[number];
  type FormTab =
    | { kind: 'evolution'; label: string }
    | { kind: 'regional';  label: string; suffix: string }
    | { kind: 'mega';      label: string; forms: SpeciesForm[] }
    | { kind: 'gmax';      label: string; forms: SpeciesForm[] }
    | { kind: 'alternate'; label: string; forms: SpeciesForm[] };

  let formTabs = $derived((): FormTab[] => {
    const tabs: FormTab[] = [{ kind: 'evolution', label: 'EVOLUTION' }];
    const seenSuffixes = new Set<string>();
    const mega: SpeciesForm[] = [];
    const gmax: SpeciesForm[] = [];
    const alt:  SpeciesForm[] = [];

    for (const f of data.speciesForms) {
      if (f.isDefault) continue;
      const suffix = REGIONAL_SUFFIXES.find(r => f.slug.includes('-' + r));
      if (suffix) {
        const after = f.slug.slice(f.slug.indexOf('-' + suffix) + 1 + suffix.length);
        if (after.length > 0) {
          if (!alt.some(x => x.slug === f.slug)) alt.push(f);
        } else if (!seenSuffixes.has(suffix)) {
          seenSuffixes.add(suffix);
          tabs.push({ kind: 'regional', label: suffix.toUpperCase(), suffix });
        }
      } else if (f.slug.includes('-gmax')) {
        if (!gmax.some(x => x.slug === f.slug)) gmax.push(f);
      } else if (f.slug.includes('-mega')) {
        if (!mega.some(x => x.slug === f.slug)) mega.push(f);
      } else {
        if (!alt.some(x => x.slug === f.slug)) alt.push(f);
      }
    }

    if (mega.length > 0) tabs.push({ kind: 'mega',      label: 'MEGA',  forms: mega });
    if (gmax.length > 0) tabs.push({ kind: 'gmax',      label: 'GMAX',  forms: gmax });
    if (alt.length  > 0) tabs.push({ kind: 'alternate', label: 'FORMS', forms: alt  });
    return tabs;
  });

  const _currentSuffix = REGIONAL_SUFFIXES.find(r => data.pokemon.slug.endsWith('-' + r));
  let activeFormTab = $state<string>(_currentSuffix ? _currentSuffix.toUpperCase() : 'EVOLUTION');
  let activeFormTabObj = $derived(formTabs().find(t => t.label === activeFormTab) ?? formTabs()[0]);

  // --- Page tabs ---
  const PAGE_TABS = [
    { key: 'info',     label: 'INFO' },
    { key: 'forms',    label: 'FORMS' },
    { key: 'learnset', label: 'LEARNSET' },
    { key: 'location', label: 'LOCATION' },
  ] as const;
  let activePageTab = $state('info');

  // --- Save handlers ---
  function handleSave() {
    saved = false;
    return async ({ result, update }: any) => {
      if (result.type === 'success') {
        saved = true;
        setTimeout(() => saved = false, 2500);
      } else {
        // failure → show error; redirect → follow new slug
        await update({ reset: false });
      }
    };
  }
</script>

<svelte:head>
  <title>Edit {data.pokemon.name} | Karta Yaxu</title>
</svelte:head>

<div class="pokemon-detail">

  <!-- ── Main update form: header + meta-strip ──────────────────────────── -->
  <form id="main-form" action="?/update" method="post" use:enhance={handleSave}>
    <!-- Hidden mirrors for INFO-tab fields -->
    <input type="hidden" name="hp"            value={hpVal} />
    <input type="hidden" name="atk"           value={atkVal} />
    <input type="hidden" name="def"           value={defVal} />
    <input type="hidden" name="spa"           value={spaVal} />
    <input type="hidden" name="spd"           value={spdVal} />
    <input type="hidden" name="spe"           value={speVal} />
    <input type="hidden" name="height"        value={heightVal} />
    <input type="hidden" name="weight"        value={weightVal} />
    <input type="hidden" name="spriteDefault" value={spriteDefaultVal} />
    <input type="hidden" name="spriteShiny"   value={spriteShinyVal} />
    <input type="hidden" name="spriteArtwork" value={spriteArtworkVal} />
    <input type="hidden" name="order"          value={orderVal} />
    <input type="hidden" name="isDefault"      value={isDefaultVal ? 'true' : ''} />
    <input type="hidden" name="slot1TypeId"    value={slot1TypeId    ?? ''} />
    <input type="hidden" name="slot2TypeId"    value={slot2TypeId    ?? ''} />
    <input type="hidden" name="slot1AbilityId" value={slot1AbilityVal ?? ''} />
    <input type="hidden" name="slot2AbilityId" value={slot2AbilityVal ?? ''} />
    <input type="hidden" name="hiddenAbilityId" value={hiddenAbilityVal ?? ''} />

    <!-- Wiki-style header card -->
    <header class="detail-header">
      <div class="dex-num">#{(currentSpecies?.dexNumber ?? 0).toString().padStart(3, '0')}</div>
      <div class="header-main">
        <input class="h1-input" name="name" type="text" maxlength="80" required bind:value={nameVal} />
        {#if currentSpecies}
          <div class="genus">{currentSpecies.name}</div>
        {/if}
      </div>
      <div class="types">
        {#if slot1Type}
          <TypeBadge name={slot1Type.name} hexColor={slot1Type.hexColor ?? null} size="lg" />
        {/if}
        {#if slot2Type}
          <TypeBadge name={slot2Type.name} hexColor={slot2Type.hexColor ?? null} size="lg" />
        {/if}
      </div>
    </header>

    <!-- Editor-only meta-strip -->
    <div class="meta-strip">
      <div class="meta-field">
        <label class="meta-label">SLUG</label>
        <input class="meta-input" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required bind:value={slugVal} />
      </div>
      <div class="meta-field">
        <label class="meta-label">SPECIES</label>
        <select class="meta-select" name="speciesId" required bind:value={speciesIdVal}>
          {#each data.species as s}
            <option value={s.id}>#{s.dexNumber} {s.name}</option>
          {/each}
        </select>
      </div>
      {#if data.pokemon.pokeapiId}
      {/if}
      <div class="meta-actions">
        <button type="submit" class="save-btn">SAVE</button>
        <a href="." class="cancel-link">CANCEL</a>
        {#if saved}<span class="save-ok">✓ SAVED</span>{/if}
        {#if form?.updateError}<span class="form-error">{form.updateError}</span>{/if}
      </div>
    </div>
  </form>

  <!-- ── Tabs ──────────────────────────────────────────────────────────── -->
  <Tabs tabs={PAGE_TABS} bind:active={activePageTab}>

  {#if activePageTab === 'info'}
  <!-- INFO tab: 2×2 content-grid -->
  <div class="content-grid">

    <!-- Top-left: Sprite -->
    <div class="sprite-cell">
      <div class="image-box">
        <div class="image-area">
          {#if currentSpriteUrl}
            <img src={currentSpriteUrl} alt={nameVal} class:shiny-active={isShiny} />
          {:else}
            <div class="no-sprite">NO SPRITE</div>
          {/if}
          <button type="button" class="shiny-toggle" onclick={() => isShiny = !isShiny}>
            {isShiny ? '◼ NORMAL' : '✦ SHINY'}
          </button>
        </div>
        <div class="sprite-inputs">
          <div class="sprite-field">
            <label class="sprite-label">DEFAULT</label>
            <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteDefaultVal} />
          </div>
          <div class="sprite-field">
            <label class="sprite-label">SHINY</label>
            <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteShinyVal} />
          </div>
          <div class="sprite-field">
            <label class="sprite-label">ARTWORK</label>
            <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteArtworkVal} />
          </div>
        </div>
      </div>
    </div>

    <!-- Top-right: Stats -->
    <div class="stats-cell">
      <InfoBox title="BASE STATS">
        <div class="stats-container">
          {#each [
            { label: 'HP',      key: 'hp',  get: () => hpVal,  set: (v: number) => { hpVal  = v; } },
            { label: 'ATTACK',  key: 'atk', get: () => atkVal, set: (v: number) => { atkVal = v; } },
            { label: 'DEFENSE', key: 'def', get: () => defVal, set: (v: number) => { defVal = v; } },
            { label: 'SP. ATK', key: 'spa', get: () => spaVal, set: (v: number) => { spaVal = v; } },
            { label: 'SP. DEF', key: 'spd', get: () => spdVal, set: (v: number) => { spdVal = v; } },
            { label: 'SPEED',   key: 'spe', get: () => speVal, set: (v: number) => { speVal = v; } },
          ] as stat}
            <div class="stat-edit-row">
              <span class="stat-edit-label">{stat.label}</span>
              <input
                class="stat-edit-input"
                type="number" min="0" max="255"
                value={stat.get()}
                oninput={(e) => stat.set(Math.max(0, Math.min(255, Number((e.currentTarget as HTMLInputElement).value))))}
              />
              <div class="stat-bar-only">
                <div class="stat-bar-fill" style="width: {(stat.get() / 255) * 100}%;"></div>
              </div>
            </div>
          {/each}
          <div class="stat-total">
            <span class="label">TOTAL:</span>
            <span class="value">{totalStats}</span>
          </div>
        </div>
      </InfoBox>
    </div>

    <!-- Bottom-left: Types + Abilities -->
    <div class="info-cell">
      <InfoBox title="TYPES">
        <div class="sub-form-inline">
          <div class="sub-field">
            <label class="sub-label">SLOT 1</label>
            <select bind:value={slot1TypeId}>
              {#each data.allTypes as t}
                <option value={t.id}>{t.name}</option>
              {/each}
            </select>
          </div>
          <div class="sub-field">
            <label class="sub-label">SLOT 2</label>
            <select bind:value={slot2TypeId}>
              <option value={null}>— none —</option>
              {#each data.allTypes as t}
                <option value={t.id}>{t.name}</option>
              {/each}
            </select>
          </div>
        </div>
      </InfoBox>

      <InfoBox title="ABILITIES">
        <div class="sub-form-inline">
          <div class="sub-field">
            <label class="sub-label">SLOT 1</label>
            <select bind:value={slot1AbilityVal}>
              {#each data.allAbilities as a}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </div>
          <div class="sub-field">
            <label class="sub-label">SLOT 2</label>
            <select bind:value={slot2AbilityVal}>
              <option value={null}>— none —</option>
              {#each data.allAbilities as a}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </div>
          <div class="sub-field">
            <label class="sub-label">HIDDEN</label>
            <select bind:value={hiddenAbilityVal}>
              <option value={null}>— none —</option>
              {#each data.allAbilities as a}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </div>
        </div>
      </InfoBox>
    </div>

    <!-- Bottom-right: Data -->
    <div class="data-cell">
      <InfoBox title="DATA">
        <ul class="data-edit-list">
          <li class="data-edit-row">
            <span class="data-label">HEIGHT</span>
            <div class="data-input-wrap">
              <input class="data-input" type="number" min="0" bind:value={heightVal} />
              <span class="data-unit">dm</span>
            </div>
          </li>
          <li class="data-edit-row">
            <span class="data-label">WEIGHT</span>
            <div class="data-input-wrap">
              <input class="data-input" type="number" min="0" bind:value={weightVal} />
              <span class="data-unit">hg</span>
            </div>
          </li>
          <li class="data-edit-row">
            <span class="data-label">ORDER</span>
            <input class="data-input" type="number" bind:value={orderVal} />
          </li>
          <li class="data-edit-row">
            <span class="data-label">ID</span>
            <span class="data-value">{data.pokemon.id}</span>
          </li>
          {#if data.pokemon.pokeapiId}
            <li class="data-edit-row">
              <span class="data-label">POKEAPI</span>
              <span class="data-value">{data.pokemon.pokeapiId}</span>
            </li>
          {/if}
          <li class="data-check-row">
            <label class="data-check-label">
              <input type="checkbox" bind:checked={isDefaultVal} />
              SPECIES DEFAULT
            </label>
          </li>
        </ul>
      </InfoBox>
    </div>

  </div><!-- end .content-grid -->


  {:else if activePageTab === 'forms'}
  <!-- FORMS tab: inner sub-tabs (EVOLUTION + regional/mega/gmax form tiles) -->

  <nav class="form-tabs-bar">
    {#each formTabs() as tab}
      <button
        class="form-tab-btn"
        class:active={activeFormTab === tab.label}
        onclick={() => activeFormTab = tab.label}
      >{tab.label}</button>
    {/each}
  </nav>

  {#if activeFormTabObj?.kind === 'evolution'}
  <!-- EVOLUTION sub-tab: editable predecessor / successors -->
  <section class="tab-section">
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
        <details class="add-toggle">
          <summary>EDIT CONDITIONS</summary>
          <form action="?/updateConditions" method="post" use:enhance class="sub-form">
            <div class="field"><label>TRIGGER</label><select name="trigger" required>{#each ['level-up','use-item','use-move','trade','three-critical-hits','shed','spin'] as t}<option value={t} selected={data.predecessorConditions?.trigger === t}>{t}</option>{/each}</select></div>
            <div class="field"><label>MIN LEVEL</label><input name="minLevel" type="number" min="0" max="100" value={data.predecessorConditions?.minLevel ?? ''}></div>
            <div class="field"><label>ITEM (use-item)</label><select name="itemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id} selected={(data.predecessorConditions?.itemId ?? null) === i.id}>{i.name}</option>{/each}</select></div>
            <div class="field"><label>HELD ITEM</label><select name="heldItemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id} selected={(data.predecessorConditions?.heldItemId ?? null) === i.id}>{i.name}</option>{/each}</select></div>
            <div class="field"><label>KNOWN MOVE</label><select name="knownMoveId"><option value="">— none —</option>{#each data.allMoves as m}<option value={m.id} selected={(data.predecessorConditions?.knownMoveId ?? null) === m.id}>{m.name}</option>{/each}</select></div>
            <div class="field"><label>KNOWN MOVE TYPE</label><select name="knownMoveTypeId"><option value="">— none —</option>{#each data.allTypes as t}<option value={t.id} selected={(data.predecessorConditions?.knownMoveTypeId ?? null) === t.id}>{t.name}</option>{/each}</select></div>
            <div class="field"><label>MIN HAPPINESS</label><input name="minHappiness" type="number" min="0" max="255" value={data.predecessorConditions?.minHappiness ?? ''}></div>
            <div class="field"><label>TIME OF DAY</label><select name="timeOfDay"><option value="">— any —</option>{#each ['day','night','dusk'] as t}<option value={t} selected={data.predecessorConditions?.timeOfDay === t}>{t}</option>{/each}</select></div>
            <div class="field"><label>GENDER</label><select name="gender"><option value="">— any —</option><option value="1" selected={data.predecessorConditions?.gender === 1}>female</option><option value="2" selected={data.predecessorConditions?.gender === 2}>male</option></select></div>
            <div class="field"><label>LOCATION</label><input name="location" type="text" value={data.predecessorConditions?.location ?? ''}></div>
            <div class="form-actions"><button type="submit" class="save-btn-sm">SAVE CONDITIONS</button></div>
          </form>
        </details>
        <form action="?/clearPredecessor" method="post" use:enhance class="inline-form">
          <button type="submit" class="delete-btn-sm" onclick={(e) => { if (!confirm('Remove predecessor?')) e.preventDefault(); }}>CLEAR PREDECESSOR</button>
        </form>
      {:else}
        <p class="evo-empty">Root of an evolution chain (no predecessor).</p>
        <details class="add-toggle">
          <summary>+ SET PREDECESSOR</summary>
          <form action="?/setPredecessor" method="post" use:enhance class="sub-form">
            <div class="field full"><label>PARENT POKÉMON</label><select name="parentPokemonId" required><option value="">— pick a parent —</option>{#each data.allPokemon as pk}<option value={pk.id}>{pk.name} ({pk.slug})</option>{/each}</select></div>
            <div class="field"><label>TRIGGER</label><select name="trigger" required>{#each ['level-up','use-item','use-move','trade','three-critical-hits','shed','spin'] as t}<option value={t}>{t}</option>{/each}</select></div>
            <div class="field"><label>MIN LEVEL</label><input name="minLevel" type="number" min="0" max="100"></div>
            <div class="field"><label>ITEM</label><select name="itemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id}>{i.name}</option>{/each}</select></div>
            <div class="field"><label>MIN HAPPINESS</label><input name="minHappiness" type="number" min="0" max="255"></div>
            <div class="field"><label>TIME OF DAY</label><select name="timeOfDay"><option value="">— any —</option>{#each ['day','night','dusk'] as t}<option value={t}>{t}</option>{/each}</select></div>
            <div class="form-actions"><button type="submit" class="save-btn-sm">SET PREDECESSOR</button></div>
          </form>
        </details>
      {/if}
    </div>
    <div class="evo-panel">
      <h4 class="evo-panel-title">SUCCESSORS ({data.successors.length})</h4>
      {#if data.successors.length === 0}
        <p class="evo-empty">No successors. To add one, edit the target pokémon and set this as its predecessor.</p>
      {:else}
        <div class="evo-successors">
          {#each data.successors as s (s.id)}
            <a href="/editor/pokemon/{s.slug}" class="evo-link card-interactive">
              → {s.name}
              {#if s.trigger}<span class="evo-conditions">{s.trigger}{#if s.minLevel}· lvl {s.minLevel}{/if}{#if s.itemId}· item {data.allItems.find(i => i.id === s.itemId)?.name ?? `#${s.itemId}`}{/if}</span>{/if}
            </a>
          {/each}
        </div>
      {/if}
    </div>
    {#if form?.evoError}<p class="form-error">{form.evoError}</p>{/if}
  </section>

  {:else}
  <!-- Form tiles for ALOLA / MEGA / GMAX / FORMS sub-tabs -->
  {@const tabForms = activeFormTabObj?.kind === 'regional'
    ? data.speciesForms.filter(f => f.slug.includes('-' + (activeFormTabObj as { suffix: string }).suffix))
    : ((activeFormTabObj as { forms?: SpeciesForm[] }).forms ?? [])}
  <section class="tab-section">
    <div class="form-tiles">
      {#each tabForms as f (f.slug)}
        <a href="/editor/pokemon/{f.slug}" class="form-tile" class:current={f.slug === data.pokemon.slug}>
          <span class="form-tile-num">{f.pokeapiId ? `#${String(f.pokeapiId).padStart(3, '0')}` : ''}</span>
          <span class="form-tile-name">{f.name.toUpperCase()}</span>
          {#if f.spriteUrl}
            <img src={f.spriteUrl} alt={f.name} class="form-tile-sprite" />
          {:else}
            <div class="form-tile-no-sprite"></div>
          {/if}
          <div class="form-tile-types">
            {#each f.types as t}
              <TypeBadge name={t.name} hexColor={t.hexColor} size="sm" />
            {/each}
          </div>
        </a>
      {/each}
    </div>
  </section>
  {/if}


  {:else if activePageTab === 'learnset'}
  <!-- LEARNSET tab -->
  <section class="tab-section">
    <div class="learnset-controls">
      <div class="field">
        <label>VERSION GROUP</label>
        <select onchange={onVgChange}>
          {#each data.vgCounts as v}
            <option value={v.slug} selected={v.id === data.selectedVgId}>{v.slug} ({v.count})</option>
          {/each}
        </select>
      </div>
      {#if data.selectedVgId != null && data.learnset.length > 0}
        <form action="?/clearLearnsetForVg" method="post" use:enhance>
          <input type="hidden" name="versionGroupId" value={data.selectedVgId}>
          <button type="submit" class="delete-btn-sm" onclick={(e) => { if (!confirm(`Clear all ${data.learnset.length} entries?`)) e.preventDefault(); }}>CLEAR THIS VG</button>
        </form>
      {/if}
      <details class="add-toggle">
        <summary>COPY FROM ANOTHER VG</summary>
        <form action="?/copyLearnsetFromVg" method="post" use:enhance class="copy-form">
          <input type="hidden" name="targetVgId" value={data.selectedVgId}>
          <select name="sourceVgId" required>
            <option value="">— pick source vg —</option>
            {#each data.vgCounts.filter(v => v.id !== data.selectedVgId && v.count > 0) as v}
              <option value={v.id}>{v.slug} ({v.count})</option>
            {/each}
          </select>
          <button type="submit" class="save-btn-sm">COPY</button>
        </form>
      </details>
    </div>
    {#if data.selectedVgId == null}
      <p class="evo-empty">No version groups with data. Add an entry below to start one.</p>
    {/if}
    {#if data.learnset.length > 0}
      <div class="learnset-table">
        <div class="learnset-head"><span>METHOD</span><span>MOVE</span><span>LEVEL</span><span></span></div>
        {#each data.learnset as entry (`${entry.moveId}-${entry.learnMethodId}-${entry.level}`)}
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
      <p class="evo-empty">No entries in this version group yet.</p>
    {/if}
    <details class="add-toggle">
      <summary>+ ADD MOVE TO LEARNSET</summary>
      <form action="?/addLearnsetEntry" method="post" use:enhance class="sub-form add-form">
        <input type="hidden" name="versionGroupId" value={data.selectedVgId}>
        <div class="field">
          <label>MOVE</label>
          <input list="all-moves-list" bind:value={moveQuery} placeholder="type to search…" autocomplete="off" required>
          <datalist id="all-moves-list">{#each data.allMoves as m}<option value={m.name}></option>{/each}</datalist>
          <input type="hidden" name="moveId" value={selectedMoveId ?? ''}>
          {#if moveQuery && selectedMoveId == null}<span class="prefill-msg">no match</span>{/if}
        </div>
        <div class="field"><label>METHOD</label><select name="learnMethodId" required>{#each data.allLearnMethods as m}<option value={m.id} selected={m.slug === 'level-up'}>{m.name}</option>{/each}</select></div>
        <div class="field"><label>LEVEL</label><input name="level" type="number" min="0" max="100" value="0"></div>
        <div class="form-actions"><button type="submit" class="save-btn-sm">ADD ENTRY</button></div>
      </form>
    </details>
    {#if form?.learnsetError}<p class="form-error">{form.learnsetError}</p>{/if}
  </section>


  {:else if activePageTab === 'location'}
  <div class="location-stub">
    <p class="evo-empty">Encounter data is read-only.</p>
    <a href="/wiki/pokemon/{data.pokemon.slug}" class="wiki-link-lg">View location data in wiki →</a>
  </div>
  {/if}

  </Tabs>

  <!-- ── Resource / Delete (outside tabs, at bottom) ───────────────────── -->
  {#if data.pokemon.source === 'edited' && data.pokemon.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import from PokeAPI? Stats, types, abilities, sprites, and learnset will be restored to canonical data.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Restores stats / types / abilities / learnset. Evolution pointer preserved.</span>
    </form>
  {/if}

  {#if data.pokemon.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn" onclick={(e) => { if (!confirm('Delete this Pokémon? Sub-tables (types, abilities, forms) removed too.')) e.preventDefault(); }}>DELETE THIS POKÉMON</button>
      <span class="delete-hint">Only custom pokémon can be deleted.</span>
    </form>
  {/if}

</div>

<style>
  .pokemon-detail {
    display: flex; flex-direction: column; gap: var(--spacing-6);
  }

  /* Header - identical to wiki */
  .detail-header {
    display: flex; align-items: center; gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: var(--shadow-card);
    flex-wrap: wrap;
  }
  .dex-num { font-family: var(--font-display); font-size: 16px; color: var(--theme-text-secondary); flex-shrink: 0; }
  .header-main { flex: 1; display: flex; flex-direction: column; gap: var(--spacing-1); }
  .h1-input {
    font-family: var(--font-display); font-size: 24px;
    color: var(--theme-text-primary); background: none;
    border: none; border-bottom: 2px solid transparent;
    width: 100%; outline: none; padding: 0;
  }
  .h1-input:focus { border-bottom-color: var(--theme-text-accent); }
  .genus { font-family: var(--font-mono); font-size: 12px; color: var(--theme-text-secondary); }
  .types { display: flex; gap: var(--spacing-2); }

  /* Meta-strip (editor-only) */
  .meta-strip {
    display: flex; align-items: center; flex-wrap: wrap; gap: var(--spacing-4);
    border-left: 4px solid var(--theme-border-primary);
    border-right: 4px solid var(--theme-border-primary);
    border-bottom: 4px solid var(--theme-border-primary);
    border-top: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-6);
    background-color: var(--theme-bg-secondary);
    box-shadow: var(--shadow-card);
    margin-top: -4px;
  }
  .meta-field { display: flex; align-items: center; gap: var(--spacing-2); }
  .meta-label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); white-space: nowrap; }
  .meta-input, .meta-select {
    background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 11px;
    padding: 2px var(--spacing-2);
  }
  .meta-input { width: 120px; }
  .wiki-link { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); text-decoration: none; }
  .wiki-link:hover { text-decoration: underline; }
  .meta-actions { display: flex; align-items: center; gap: var(--spacing-3); margin-left: auto; }

  /* Source badges */
  .src-badge { font-family: var(--font-display); font-size: 8px; padding: 2px 6px; border: 2px solid var(--theme-border-primary); }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  /* Shared button styles */
  .save-btn {
    font-family: var(--font-display); font-size: 10px; color: var(--theme-text-primary);
    background-color: var(--theme-bg-secondary); border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2) var(--spacing-4); cursor: pointer;
    box-shadow: var(--shadow-card-subtle); transition: all 0.1s ease;
  }
  .save-btn:hover { background-color: var(--theme-bg-hover); transform: translate(-1px, -1px); box-shadow: var(--shadow-card-hover); }
  .save-btn-sm {
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-primary);
    background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    padding: 2px var(--spacing-3); cursor: pointer; transition: all 0.1s ease;
  }
  .save-btn-sm:hover { background-color: var(--theme-bg-hover); }
  .cancel-link { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-secondary); text-decoration: none; }
  .cancel-link:hover { color: var(--theme-text-primary); }
  .save-ok { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-accent); }
  .save-ok-sm { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-accent); }
  .form-error { font-family: var(--font-mono); font-size: 10px; color: var(--color-text-error); }
  .form-error-sm { font-family: var(--font-mono); font-size: 9px; color: var(--color-text-error); }
  .delete-btn-sm {
    font-family: var(--font-display); font-size: 9px; color: var(--color-text-error);
    background: none; border: 2px solid var(--color-text-error);
    padding: 2px var(--spacing-3); cursor: pointer;
  }

  /* Content grid - identical to wiki */
  .content-grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-6);
  }
  .sprite-cell { display: flex; flex-direction: column; }
  .stats-cell { display: flex; flex-direction: column; }
  .info-cell { display: flex; flex-direction: column; gap: var(--spacing-4); }
  .data-cell { display: flex; flex-direction: column; }

  /* Image box — column layout: image area on top, sprite inputs on bottom */
  .image-box {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary); box-shadow: var(--shadow-card);
    display: flex; flex-direction: column;
    transition: background-color 0.15s ease;
    flex: 1;
  }
  .image-area {
    position: relative;
    display: flex; justify-content: center; align-items: center;
    padding: var(--spacing-6); flex: 1; min-height: 240px;
    transition: background-color 0.15s ease;
  }
  .image-box:has(.shiny-active) .image-area { background-color: color-mix(in srgb, var(--theme-bg-secondary) 85%, #b39ddb 15%); }
  .image-box img { max-width: 100%; max-height: 240px; object-fit: contain; }
  .no-sprite { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-muted); }
  .shiny-toggle {
    position: absolute; bottom: var(--spacing-2); right: var(--spacing-2);
    font-family: var(--font-display); font-size: 8px; padding: 4px 8px;
    background-color: var(--theme-bg-primary); color: var(--theme-text-secondary);
    border: 2px solid var(--theme-border-primary); cursor: pointer;
    box-shadow: 2px 2px 0px var(--theme-border-primary); transition: all 0.05s ease;
  }
  .shiny-toggle:hover { color: var(--theme-text-accent); border-color: var(--theme-text-accent); transform: translate(-1px, -1px); }

  /* Sprite URL inputs — docked inside image-box */
  .sprite-inputs {
    display: flex; flex-direction: column; gap: var(--spacing-2);
    padding: var(--spacing-3);
    border-top: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-primary);
  }
  .sprite-field { display: flex; align-items: center; gap: var(--spacing-2); }
  .sprite-label { font-family: var(--font-display); font-size: 8px; color: var(--theme-text-muted); width: 60px; flex-shrink: 0; }
  .sprite-url-input {
    flex: 1; background-color: var(--theme-bg-secondary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-secondary); font-family: var(--font-mono); font-size: 10px;
    padding: 2px var(--spacing-2);
  }

  /* Stat editing rows */
  .stats-container { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .stat-edit-row { display: flex; align-items: center; gap: var(--spacing-2); }
  .stat-edit-label { width: 60px; font-family: var(--font-display); font-size: 8px; color: var(--theme-text-secondary); text-align: right; flex-shrink: 0; }
  .stat-edit-input {
    width: 44px; background: none; border: none; border-bottom: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 12px;
    font-weight: bold; text-align: center; outline: none; flex-shrink: 0;
  }
  .stat-edit-input:focus { border-bottom-color: var(--theme-text-accent); }
  .stat-bar-only {
    flex: 1;
    height: 10px;
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
  }
  .stat-bar-fill {
    height: 100%;
    background-color: var(--theme-text-accent);
  }
  .stat-total { display: flex; justify-content: flex-end; gap: var(--spacing-3); margin-top: var(--spacing-2); font-family: var(--font-display); font-size: 10px; color: var(--theme-text-secondary); }
  .stat-total .value { color: var(--theme-text-primary); font-family: var(--font-mono); }
  .stat-save-row { display: flex; align-items: center; gap: var(--spacing-3); margin-top: var(--spacing-2); }
  .save-btn-sm {
    font-family: var(--font-display); font-size: 10px;
    padding: 2px var(--spacing-3);
    background: none; border: 2px solid var(--theme-text-accent); color: var(--theme-text-accent);
    cursor: pointer;
  }
  .save-btn-sm:hover { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }

  /* Data cell */
  .data-edit-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: var(--spacing-3); }
  .data-edit-row { display: flex; align-items: center; gap: var(--spacing-3); }
  .data-label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); width: 60px; flex-shrink: 0; }
  .data-value { font-family: var(--font-mono); font-size: 12px; color: var(--theme-text-primary); }
  .data-input-wrap { display: flex; align-items: center; gap: var(--spacing-2); }
  .data-input {
    width: 70px; background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 12px;
    padding: 2px var(--spacing-2);
  }
  .data-unit { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .data-check-row { display: flex; align-items: center; }
  .data-check-label { display: flex; align-items: center; gap: var(--spacing-2); font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); cursor: pointer; }

  /* Sub-forms inside InfoBox (types, abilities) */
  .sub-form-inline { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .sub-field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .sub-label { font-family: var(--font-display); font-size: 8px; color: var(--theme-text-muted); }
  .sub-field select {
    background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 11px;
    padding: 2px var(--spacing-2);
  }
  .sub-actions { display: flex; align-items: center; gap: var(--spacing-3); margin-top: var(--spacing-1); }

  /* Tab sections */
  .tab-section { display: flex; flex-direction: column; gap: var(--spacing-4); }
  .section-title { font-family: var(--font-display); font-size: 12px; color: var(--theme-text-primary); margin: 0; border-bottom: 2px solid var(--theme-border-primary); padding-bottom: var(--spacing-2); }
  .evo-panel { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .evo-panel-title { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-muted); margin: 0; }
  .evo-empty { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); margin: 0; }
  .evo-link { font-family: var(--font-mono); font-size: 12px; color: var(--theme-text-accent); text-decoration: none; }
  .evo-conditions { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); margin-left: var(--spacing-2); }
  .evo-successors { display: flex; flex-direction: column; gap: var(--spacing-2); }
  .evo-row { display: flex; align-items: center; gap: var(--spacing-2); }
  .card-interactive { display: block; padding: var(--spacing-2); border: 2px solid var(--theme-border-primary); transition: all 0.1s ease; }
  .card-interactive:hover { background-color: var(--theme-bg-hover); transform: translate(-1px, -1px); }

  /* FORMS tab inner sub-tabs */
  .form-tabs-bar {
    display: flex; gap: 0; border-bottom: 2px solid var(--theme-border-primary);
    margin-bottom: var(--spacing-4);
  }
  .form-tab-btn {
    font-family: var(--font-display); font-size: 10px; letter-spacing: 0.05em;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: none; border-bottom: 3px solid transparent;
    color: var(--theme-text-muted); cursor: pointer; margin-bottom: -2px;
  }
  .form-tab-btn:hover { color: var(--theme-text-primary); }
  .form-tab-btn.active { color: var(--theme-text-accent); border-bottom-color: var(--theme-text-accent); }

  /* Form tiles */
  .form-tiles { display: flex; flex-wrap: wrap; gap: var(--spacing-3); }
  .form-tile {
    display: flex; flex-direction: column; align-items: center; gap: var(--spacing-1);
    padding: var(--spacing-3);
    background: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    text-decoration: none; color: var(--theme-text-primary);
    min-width: 110px;
  }
  .form-tile:hover { border-color: var(--theme-text-accent); }
  .form-tile.current { border-color: var(--theme-text-accent); }
  .form-tile-num { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .form-tile-name { font-family: var(--font-display); font-size: 9px; text-align: center; }
  .form-tile-sprite { width: 80px; height: 80px; object-fit: contain; image-rendering: pixelated; }
  .form-tile-no-sprite { width: 80px; height: 80px; }
  .form-tile-types { display: flex; gap: var(--spacing-1); flex-wrap: wrap; justify-content: center; }

  .default-flag { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); margin-left: var(--spacing-2); }
  .default-badge { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); letter-spacing: 0.05em; }
  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input[type=text], .field input[type=number], .field select {
    background-color: var(--theme-bg-secondary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 11px;
    padding: var(--spacing-1) var(--spacing-2);
  }
  .field.full { grid-column: 1 / -1; }
  .field-check { flex-direction: row; align-items: center; }
  .field-check label { display: flex; align-items: center; gap: var(--spacing-2); flex-direction: row; }
  .add-toggle { margin-top: var(--spacing-2); }
  .add-toggle summary { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-accent); cursor: pointer; }
  .add-form { margin-top: var(--spacing-3); }

  /* Learnset styles */
  .learnset-controls { display: flex; align-items: center; gap: var(--spacing-4); flex-wrap: wrap; }
  .learnset-table { display: flex; flex-direction: column; }
  .learnset-head { display: grid; grid-template-columns: 120px 1fr 60px 60px; gap: var(--spacing-3); padding: var(--spacing-2) var(--spacing-3); background-color: var(--theme-bg-primary); border-bottom: 2px solid var(--theme-border-primary); font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .learnset-row { display: grid; grid-template-columns: 120px 1fr 60px 60px; gap: var(--spacing-3); padding: var(--spacing-2) var(--spacing-3); border-bottom: 1px solid var(--theme-border-primary); align-items: center; }
  .learnset-method { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
  .learnset-move { font-family: var(--font-mono); font-size: 12px; color: var(--theme-text-accent); text-decoration: none; }
  .learnset-level { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }
  .copy-form { display: flex; align-items: center; gap: var(--spacing-2); margin-top: var(--spacing-2); }
  .prefill-msg { font-family: var(--font-mono); font-size: 10px; color: var(--color-text-error); }
  .sub-form { display: flex; flex-direction: column; gap: var(--spacing-3); margin-top: var(--spacing-3); }
  .form-actions { display: flex; align-items: center; gap: var(--spacing-3); }
  .inline-form { display: inline; }

  /* Location stub */
  .location-stub { display: flex; flex-direction: column; gap: var(--spacing-4); align-items: flex-start; }
  .wiki-link-lg { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-accent); text-decoration: none; }
  .wiki-link-lg:hover { text-decoration: underline; }

  /* Resource / delete */
  .resource-form { display: flex; align-items: center; gap: var(--spacing-4); padding: var(--spacing-3); border: 2px dashed var(--theme-border-primary); }
  .resource-btn { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-secondary); background: none; border: 2px solid var(--theme-border-primary); padding: var(--spacing-2) var(--spacing-4); cursor: pointer; transition: all 0.1s ease; }
  .resource-btn:hover { color: var(--theme-text-primary); }
  .resource-hint { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .delete-form { display: flex; align-items: center; gap: var(--spacing-4); padding: var(--spacing-3); border: 2px dashed var(--color-text-error); }
  .delete-btn { font-family: var(--font-display); font-size: 10px; color: var(--color-text-error); background: none; border: 2px solid var(--color-text-error); padding: var(--spacing-2) var(--spacing-4); cursor: pointer; }
  .delete-hint { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }
  .empty-state { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
</style>
