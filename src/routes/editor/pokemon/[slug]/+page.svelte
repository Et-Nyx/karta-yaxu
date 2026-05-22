<script lang="ts">
  import { enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import StatBar from '$lib/components/StatBar.svelte';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import MoveCategoryIcon from '$lib/components/MoveCategoryIcon.svelte';
  import PokemonSpriteViewer from '$lib/components/PokemonSpriteViewer.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import PokemonDataList from '$lib/components/PokemonDataList.svelte';
  import { pageSource } from '$lib/stores/pageSource';

  let { data, form } = $props();

  // Version Mechanics (Flavor Texts)
  let versionIdsWithData = $derived(new Set([
    ...data.flavorTexts.map(f => f.versionGroupId),
    ...data.vgCounts.filter(v => v.count > 0).map(v => v.id)
  ]));

  let validVersions = $derived(data.availableVersions.filter(v => versionIdsWithData.has(v.id)));
  let selectedVersionId = $state<number | null>(null);

  $effect(() => {
    // Reactively reset selectedVersionId on page transition
    data;
    selectedVersionId = null;
  });

  $effect(() => {
    if (selectedVersionId === null && validVersions.length > 0) {
      const frlg = validVersions.find(v => v.name === 'firered-leafgreen');
      const withFlavor = new Set(data.flavorTexts.map(f => f.versionGroupId));
      const latestWithFlavor = [...validVersions].reverse().find(v => withFlavor.has(v.id));
      selectedVersionId = (frlg ?? latestWithFlavor ?? validVersions[validVersions.length - 1]).id;
    }
  });

  let isCustomVg = $derived(() => {
    if (!selectedVersionId) return false;
    const vg = data.availableVersions.find(v => v.id === selectedVersionId);
    return vg ? vg.source === 'custom' : false;
  });

  // Local state for the editable flavor text
  let flavorTextVal = $state('');

  $effect(() => {
    if (selectedVersionId) {
      const entry = data.flavorTexts.find(f => f.versionGroupId === selectedVersionId);
      flavorTextVal = entry ? entry.flavorText.replace(/\f/g, ' ') : '';
    } else {
      flavorTextVal = '';
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement).tagName;
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;
    if (!data.nav) return;
    if (e.key === 'ArrowLeft' && data.nav.prev) {
      goto(`/editor/pokemon/${data.nav.prev.pokemonSlug}?dex=${data.nav.selectedDex.slug}`);
    } else if (e.key === 'ArrowRight' && data.nav.next) {
      goto(`/editor/pokemon/${data.nav.next.pokemonSlug}?dex=${data.nav.selectedDex.slug}`);
    }
  }

  $effect(() => { pageSource.set(data.pokemon.source); });

  // Save states
  let saved = $state(false);

  // --- Main update form state ---
  let nameVal = $state(data.pokemon.name);
  let slugVal = $derived(
    nameVal
      .toLowerCase()
      .trim()
      .replace(/[^\w\s-]/g, '')
      .replace(/[\s_-]+/g, '-')
      .replace(/^-+|-+$/g, '')
  );

  let speciesIdVal = $state(data.pokemon.speciesId);
  let heightVal = $state(data.pokemon.height);
  let weightVal = $state(data.pokemon.weight);
  let isDefaultVal = $derived(data.pokemon.isDefault);

  let spriteDefaultVal = $state(data.pokemon.spriteDefault ?? '');
  let spriteShinyVal = $state(data.pokemon.spriteShiny ?? '');
  let spriteArtworkVal = $state(data.pokemon.spriteArtwork ?? '');
  let spriteArtworkShinyVal = $state(data.pokemon.spriteArtworkShiny ?? '');
  let spriteBackDefaultVal = $state(data.pokemon.spriteBackDefault ?? '');
  let spriteBackShinyVal = $state(data.pokemon.spriteBackShiny ?? '');
  let isShiny = $state(false);
  let activeImageTab = $state<'artwork' | 'front' | 'back'>('artwork');
  let copySourceVgId = $state('');

  let defaultFileSelected = $state(false);
  let shinyFileSelected = $state(false);
  let artworkFileSelected = $state(false);
  let artworkShinyFileSelected = $state(false);
  let backDefaultFileSelected = $state(false);
  let backShinyFileSelected = $state(false);

  function handleFileSelected(e: Event, type: 'default' | 'shiny' | 'artwork' | 'artwork-shiny' | 'back-default' | 'back-shiny') {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) {
      if (type === 'default') defaultFileSelected = true;
      if (type === 'shiny') shinyFileSelected = true;
      if (type === 'artwork') artworkFileSelected = true;
      if (type === 'artwork-shiny') artworkShinyFileSelected = true;
      if (type === 'back-default') backDefaultFileSelected = true;
      if (type === 'back-shiny') backShinyFileSelected = true;

      const objectUrl = URL.createObjectURL(file);
      if (type === 'default') spriteDefaultVal = objectUrl;
      if (type === 'shiny') spriteShinyVal = objectUrl;
      if (type === 'artwork') spriteArtworkVal = objectUrl;
      if (type === 'artwork-shiny') spriteArtworkShinyVal = objectUrl;
      if (type === 'back-default') spriteBackDefaultVal = objectUrl;
      if (type === 'back-shiny') spriteBackShinyVal = objectUrl;
    }
  }

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

  // --- Species detailed derived helpers ---
  let genderText = $derived(() => {
    const gr = data.currentSpeciesDetail?.genderRate;
    if (gr === null || gr === undefined) return 'UNKNOWN';
    if (gr === -1)   return 'GENDERLESS';
    if (gr === 0)    return '100% ♂';
    if (gr === 8)    return '100% ♀';
    const femaleChance = Math.round((gr / 8) * 100);
    return `${100 - femaleChance}% ♂ / ${femaleChance}% ♀`;
  });

  // --- Learnset ---
  let moveQuery = $state('');
  let selectedMoveId = $derived(
    data.allMoves.find(m => m.name.toLowerCase() === moveQuery.toLowerCase())?.id ?? null
  );

  function onVgChange(val: string | Event) {
    const newSlug = typeof val === 'string' ? val : (val.target as HTMLSelectElement).value;
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

  // --- Evo tree helpers ---
  function evoCondText(cond: typeof data.evoChain[number]['conditions']): string {
    if (!cond) return '';
    if (cond.trigger === 'level-up') {
      if (cond.minLevel)     return `Lv. ${cond.minLevel}`;
      if (cond.minHappiness) return `Happiness ${cond.minHappiness}`;
      if (cond.timeOfDay)    return `Level up (${cond.timeOfDay})`;
      return 'Level up';
    }
    if (cond.trigger === 'use-item') return cond.itemName ?? 'Use item';
    if (cond.trigger === 'trade')    return 'Trade';
    return cond.trigger;
  }

  let evoTreeData = $derived(() => {
    const nodeMap = new Map(data.evoChain.map(n => [n.id, n]));
    const childrenOf = new Map<number, typeof data.evoChain>();
    for (const n of data.evoChain) {
      if (n.evolvesFromId == null) continue;
      if (!childrenOf.has(n.evolvesFromId)) childrenOf.set(n.evolvesFromId, []);
      childrenOf.get(n.evolvesFromId)!.push(n);
    }
    const chainIdSet = new Set(data.evoChain.map(n => n.id));
    const roots = data.evoChain.filter(n => n.evolvesFromId == null || !chainIdSet.has(n.evolvesFromId));
    return { nodeMap, childrenOf, roots };
  });

  const TRIGGERS = ['level-up','use-item','use-move','trade','three-critical-hits','shed','spin'] as const;

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

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <title>Edit {data.pokemon.name} | Karta Yaxu</title>
</svelte:head>

<div class="pokemon-detail">

  <!-- ── Main update form: header + meta-strip ──────────────────────────── -->
  <form id="main-form" action="?/update" method="post" enctype="multipart/form-data" use:enhance={handleSave}>
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
    <input type="hidden" name="spriteArtworkShiny" value={spriteArtworkShinyVal} />
    <input type="hidden" name="spriteBackDefault"  value={spriteBackDefaultVal} />
    <input type="hidden" name="spriteBackShiny"    value={spriteBackShinyVal} />
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
        {#if data.genus}
          <div class="genus">{data.genus}</div>
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
      <input type="hidden" name="slug" value={slugVal} />
      <div class="meta-field">
        <label class="meta-label">SPECIES</label>
        <RetroSelect
          name="speciesId"
          bind:value={speciesIdVal}
          options={data.species.map(s => ({ value: s.id, label: `#${s.dexNumber} ${s.name}` }))}
          variant="editor"
        />
      </div>
      <div class="meta-actions">
        <button type="submit" class="save-btn">SAVE</button>
        <a href="." class="cancel-link">CANCEL</a>
        {#if saved}<span class="save-ok">✓ SAVED</span>{/if}
        {#if form?.updateError}<span class="form-error">{form.updateError}</span>{/if}
      </div>
    </div>
  </form>

  <div class="meta-row">
    {#if validVersions.length > 0}
      <VersionSelector
        bind:value={selectedVersionId}
        options={validVersions.map(v => ({ value: v.id, label: v.name.toUpperCase().replace(/-/g, ' ') }))}
      />
    {/if}

    {#if data.nav}
      <nav class="dex-nav">
        {#if data.nav.prev}
          <a
            href="/editor/pokemon/{data.nav.prev.pokemonSlug}?dex={data.nav.selectedDex.slug}"
            class="nav-link nav-prev"
          >← <span class="nav-text">{data.nav.prev.speciesName.toUpperCase()}</span></a>
        {:else}
          <span class="nav-disabled">←</span>
        {/if}

        <div class="nav-center">
          <span class="nav-entry">#{data.nav.currentEntry}</span>
          <RetroSelect
            value={data.nav.selectedDex.slug}
            options={data.nav.dexList.map(dex => ({ value: dex.slug, label: dex.name.toUpperCase().replace(/-/g, ' ') }))}
            onchange={(val) => goto(`?dex=${val}`)}
            class="dex-select"
          />
        </div>

        {#if data.nav.next}
          <a
            href="/editor/pokemon/{data.nav.next.pokemonSlug}?dex={data.nav.selectedDex.slug}"
            class="nav-link nav-next"
          ><span class="nav-text">{data.nav.next.speciesName.toUpperCase()}</span> →</a>
        {:else}
          <span class="nav-disabled">→</span>
        {/if}
      </nav>
    {/if}
  </div>

  {#snippet evoNode(nodeId: number)}
    {@const node = evoTreeData().nodeMap.get(nodeId)}
    {@const children = evoTreeData().childrenOf.get(nodeId) ?? []}
    {#if node}
      <div class="evo-node-wrap">
        <PokemonCard
          pokemon={{
            pokeapiId: node.pokeapiId,
            name: node.name,
            slug: node.slug,
            spriteUrl: node.spriteUrl,
            dexNumber: node.dexNumber,
            types: node.types,
          }}
          href="/editor/pokemon/{node.slug}"
          highlighted={node.id === data.pokemon.id}
        />
        {#if children.length === 1}
          {@const child = children[0]}
          {@const ct = evoCondText(child.conditions)}
          {@const childIsCurrent = child.id === data.pokemon.id}
          <div class="evo-conn-single">
            {#if ct}<span class="evo-method-tag">{ct}</span>{/if}
            <span class="evo-down-arrow">↓</span>
            {#if childIsCurrent}
              <div class="evo-conn-actions">
                <details class="evo-conn-edit">
                  <summary class="evo-edit-btn">✎</summary>
                  <form action="?/updateConditions" method="post" use:enhance class="evo-inline-form">
                    <div class="field"><label>TRIGGER</label>
                      <select name="trigger" required>
                        {#each TRIGGERS as t}<option value={t} selected={data.predecessorConditions?.trigger === t}>{t}</option>{/each}
                      </select>
                    </div>
                    <div class="field"><label>MIN LEVEL</label><input name="minLevel" type="number" min="0" max="100" value={data.predecessorConditions?.minLevel ?? ''}></div>
                    <div class="field"><label>ITEM</label>
                      <select name="itemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id} selected={(data.predecessorConditions?.itemId ?? null) === i.id}>{i.name}</option>{/each}</select>
                    </div>
                    <div class="field"><label>HELD ITEM</label>
                      <select name="heldItemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id} selected={(data.predecessorConditions?.heldItemId ?? null) === i.id}>{i.name}</option>{/each}</select>
                    </div>
                    <div class="field"><label>KNOWN MOVE</label>
                      <select name="knownMoveId"><option value="">— none —</option>{#each data.allMoves as m}<option value={m.id} selected={(data.predecessorConditions?.knownMoveId ?? null) === m.id}>{m.name}</option>{/each}</select>
                    </div>
                    <div class="field"><label>MIN HAPPINESS</label><input name="minHappiness" type="number" min="0" max="255" value={data.predecessorConditions?.minHappiness ?? ''}></div>
                    <div class="field"><label>TIME OF DAY</label>
                      <select name="timeOfDay"><option value="">— any —</option>{#each ['day','night','dusk'] as t}<option value={t} selected={data.predecessorConditions?.timeOfDay === t}>{t}</option>{/each}</select>
                    </div>
                    <div class="field"><label>GENDER</label>
                      <select name="gender"><option value="">— any —</option><option value="1" selected={data.predecessorConditions?.gender === 1}>female</option><option value="2" selected={data.predecessorConditions?.gender === 2}>male</option></select>
                    </div>
                    <div class="field"><label>LOCATION</label><input name="location" type="text" value={data.predecessorConditions?.location ?? ''}></div>
                    <button type="submit" class="save-btn-sm">SAVE</button>
                  </form>
                </details>
                <form action="?/clearPredecessor" method="post" use:enhance class="inline-form">
                  <button type="submit" class="evo-unlink-btn"
                    onclick={(e) => { if (!confirm('Remove predecessor?')) e.preventDefault(); }}>×</button>
                </form>
              </div>
            {/if}
          </div>
          {@render evoNode(child.id)}
        {:else if children.length > 1}
          <div class="evo-chunks-wrap">
            <div class="evo-stem"></div>
            <div class="evo-branch-row">
              {#each children as child, i}
                {@const ct = evoCondText(child.conditions)}
                {@const childIsCurrent = child.id === data.pokemon.id}
                <div class="evo-branch-child"
                  class:first-branch={i === 0}
                  class:last-branch={i === children.length - 1}
                  class:mid-branch={i > 0 && i < children.length - 1}>
                  <div class="evo-conn-cell">
                    {#if ct}<span class="evo-method-tag">{ct}</span>{/if}
                    <span class="evo-down-arrow">↓</span>
                    {#if childIsCurrent}
                      <div class="evo-conn-actions">
                        <details class="evo-conn-edit">
                          <summary class="evo-edit-btn">✎</summary>
                          <form action="?/updateConditions" method="post" use:enhance class="evo-inline-form">
                            <div class="field"><label>TRIGGER</label>
                              <select name="trigger" required>
                                {#each TRIGGERS as t}<option value={t} selected={data.predecessorConditions?.trigger === t}>{t}</option>{/each}
                              </select>
                            </div>
                            <div class="field"><label>MIN LEVEL</label><input name="minLevel" type="number" min="0" max="100" value={data.predecessorConditions?.minLevel ?? ''}></div>
                            <div class="field"><label>ITEM</label>
                              <select name="itemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id} selected={(data.predecessorConditions?.itemId ?? null) === i.id}>{i.name}</option>{/each}</select>
                            </div>
                            <div class="field"><label>MIN HAPPINESS</label><input name="minHappiness" type="number" min="0" max="255" value={data.predecessorConditions?.minHappiness ?? ''}></div>
                            <div class="field"><label>TIME OF DAY</label>
                              <select name="timeOfDay"><option value="">— any —</option>{#each ['day','night','dusk'] as t}<option value={t} selected={data.predecessorConditions?.timeOfDay === t}>{t}</option>{/each}</select>
                            </div>
                            <button type="submit" class="save-btn-sm">SAVE</button>
                          </form>
                        </details>
                        <form action="?/clearPredecessor" method="post" use:enhance class="inline-form">
                          <button type="submit" class="evo-unlink-btn"
                            onclick={(e) => { if (!confirm('Remove predecessor?')) e.preventDefault(); }}>×</button>
                        </form>
                      </div>
                    {/if}
                  </div>
                  {@render evoNode(child.id)}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  {/snippet}

  {#snippet spriteInputs(activeTab: 'artwork' | 'front' | 'back')}
    {#if activeTab === 'artwork'}
      <div class="sprite-field">
        <label class="sprite-label">Artwork</label>
        <div class="sprite-input-row">
          <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteArtworkVal} />
          <label class="retro-upload-btn" class:selected={artworkFileSelected}>
            {artworkFileSelected ? 'READY' : 'UPLOAD'}
            <input type="file" accept="image/*" form="main-form" name="spriteArtworkFile" class="hidden-file-input" onchange={(e) => handleFileSelected(e, 'artwork')} />
          </label>
        </div>
      </div>
      <div class="sprite-field">
        <label class="sprite-label">Shiny Artwork</label>
        <div class="sprite-input-row">
          <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteArtworkShinyVal} />
          <label class="retro-upload-btn" class:selected={artworkShinyFileSelected}>
            {artworkShinyFileSelected ? 'READY' : 'UPLOAD'}
            <input type="file" accept="image/*" form="main-form" name="spriteArtworkShinyFile" class="hidden-file-input" onchange={(e) => handleFileSelected(e, 'artwork-shiny')} />
          </label>
        </div>
      </div>
    {:else if activeTab === 'front'}
      <div class="sprite-field">
        <label class="sprite-label">Front</label>
        <div class="sprite-input-row">
          <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteDefaultVal} />
          <label class="retro-upload-btn" class:selected={defaultFileSelected}>
            {defaultFileSelected ? 'READY' : 'UPLOAD'}
            <input type="file" accept="image/*" form="main-form" name="spriteDefaultFile" class="hidden-file-input" onchange={(e) => handleFileSelected(e, 'default')} />
          </label>
        </div>
      </div>
      <div class="sprite-field">
        <label class="sprite-label">Shiny Front</label>
        <div class="sprite-input-row">
          <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteShinyVal} />
          <label class="retro-upload-btn" class:selected={shinyFileSelected}>
            {shinyFileSelected ? 'READY' : 'UPLOAD'}
            <input type="file" accept="image/*" form="main-form" name="spriteShinyFile" class="hidden-file-input" onchange={(e) => handleFileSelected(e, 'shiny')} />
          </label>
        </div>
      </div>
    {:else if activeTab === 'back'}
      <div class="sprite-field">
        <label class="sprite-label">Back</label>
        <div class="sprite-input-row">
          <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteBackDefaultVal} />
          <label class="retro-upload-btn" class:selected={backDefaultFileSelected}>
            {backDefaultFileSelected ? 'READY' : 'UPLOAD'}
            <input type="file" accept="image/*" form="main-form" name="spriteBackDefaultFile" class="hidden-file-input" onchange={(e) => handleFileSelected(e, 'back-default')} />
          </label>
        </div>
      </div>
      <div class="sprite-field">
        <label class="sprite-label">Shiny Back</label>
        <div class="sprite-input-row">
          <input class="sprite-url-input" type="text" placeholder="URL…" bind:value={spriteBackShinyVal} />
          <label class="retro-upload-btn" class:selected={backShinyFileSelected}>
            {backShinyFileSelected ? 'READY' : 'UPLOAD'}
            <input type="file" accept="image/*" form="main-form" name="spriteBackShinyFile" class="hidden-file-input" onchange={(e) => handleFileSelected(e, 'back-shiny')} />
          </label>
        </div>
      </div>
    {/if}
  {/snippet}

  <!-- ── Tabs ──────────────────────────────────────────────────────────── -->
  <Tabs tabs={PAGE_TABS} bind:active={activePageTab}>

  {#if activePageTab === 'info'}
  <!-- INFO tab: 2×2 content-grid -->
  <div class="content-grid">

    <!-- Top-left: Sprite -->
    <div class="sprite-cell">
      <PokemonSpriteViewer
        spriteDefault={spriteDefaultVal}
        spriteShiny={spriteShinyVal}
        spriteArtwork={spriteArtworkVal}
        spriteArtworkShiny={spriteArtworkShinyVal}
        spriteBackDefault={spriteBackDefaultVal}
        spriteBackShiny={spriteBackShinyVal}
        pokeapiId={data.pokemon.pokeapiId}
        name={nameVal}
        bind:activeTab={activeImageTab}
        bind:isShiny={isShiny}
        inputsSnippet={spriteInputs}
      />
    </div>

    <!-- Top-right: Stats -->
    <div class="stats-cell">
      <InfoBox title="BASE STATS">
        <div class="stats-container">
          <StatBar name="hp" bind:value={hpVal} max={255} editable={true} />
          <StatBar name="attack" bind:value={atkVal} max={255} editable={true} />
          <StatBar name="defense" bind:value={defVal} max={255} editable={true} />
          <StatBar name="special-attack" bind:value={spaVal} max={255} editable={true} />
          <StatBar name="special-defense" bind:value={spdVal} max={255} editable={true} />
          <StatBar name="speed" bind:value={speVal} max={255} editable={true} />
          <div class="stat-total">
            <span class="label">TOTAL:</span>
            <span class="value">{totalStats}</span>
          </div>
        </div>
      </InfoBox>
    </div>

    <!-- Bottom-left: Types + Abilities -->
    <div class="info-cell">
      {#if selectedVersionId}
        <div class="flavor-text-editor-section">
          <input
            form="main-form"
            type="hidden"
            name="selectedVgId"
            value={selectedVersionId}
          />
          <FlavorTextBox
            text={flavorTextVal}
            editable={isCustomVg()}
            onchange={(val) => flavorTextVal = val}
            form="main-form"
            name="flavorText"
          />
        </div>
      {/if}

      <InfoBox title="ABILITIES">
        <div class="sub-form-inline">
          <div class="sub-field">
            <label class="sub-label">SLOT 1</label>
            <RetroSelect
              bind:value={slot1AbilityVal}
              options={data.allAbilities.map(a => ({ value: a.id, label: a.name.toUpperCase() }))}
              variant="editor"
            />
          </div>
          <div class="sub-field">
            <label class="sub-label">SLOT 2</label>
            <RetroSelect
              bind:value={slot2AbilityVal}
              options={[
                { value: null, label: '— NONE —' },
                ...data.allAbilities.map(a => ({ value: a.id, label: a.name.toUpperCase() }))
              ]}
              variant="editor"
            />
          </div>
          <div class="sub-field">
            <label class="sub-label">HIDDEN</label>
            <RetroSelect
              bind:value={hiddenAbilityVal}
              options={[
                { value: null, label: '— NONE —' },
                ...data.allAbilities.map(a => ({ value: a.id, label: a.name.toUpperCase() }))
              ]}
              variant="editor"
            />
          </div>
        </div>
      </InfoBox>

      <InfoBox title="TYPES">
        <div class="sub-form-inline">
          <div class="sub-field">
            <label class="sub-label">SLOT 1</label>
            <RetroSelect
              bind:value={slot1TypeId}
              options={data.allTypes.map(t => ({ value: t.id, label: t.name.toUpperCase() }))}
              variant="editor"
            />
          </div>
          <div class="sub-field">
            <label class="sub-label">SLOT 2</label>
            <RetroSelect
              bind:value={slot2TypeId}
              options={[
                { value: null, label: '— NONE —' },
                ...data.allTypes.map(t => ({ value: t.id, label: t.name.toUpperCase() }))
              ]}
              variant="editor"
            />
          </div>
        </div>
      </InfoBox>
    </div>

    <!-- Bottom-right: Data -->
    <div class="data-cell">
      <InfoBox title="DATA">
        <!-- Hidden inputs to submit height & weight from parent state -->
        <input type="hidden" name="height" value={heightVal} />
        <input type="hidden" name="weight" value={weightVal} />

        <PokemonDataList
          bind:height={heightVal}
          bind:weight={weightVal}
          captureRate={data.currentSpeciesDetail?.captureRate ?? null}
          genderText={genderText()}
          eggGroups={data.currentEggGroups.map(e => e.name.toUpperCase()).join(', ') || '—'}
          habitat={data.currentSpeciesDetail?.habitat ?? null}
          growthRate={data.currentSpeciesDetail?.growthRate ?? null}
          happiness={data.currentSpeciesDetail?.baseHappiness ?? null}
          pokemonId={data.pokemon.id}
          isDefault={isDefaultVal}
          speciesSlug={data.currentSpeciesDetail?.slug ?? null}
          editable={true}
        />

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
  <section class="tab-section">

    {#if !data.predecessor}
    <details class="evo-add-pred">
      <summary>+ SET PREDECESSOR</summary>
      <form action="?/setPredecessor" method="post" use:enhance class="evo-inline-form">
        <div class="field full"><label>PARENT POKÉMON</label>
          <select name="parentPokemonId" required>
            <option value="">— pick —</option>
            {#each data.allPokemon as pk}<option value={pk.id}>{pk.name} ({pk.slug})</option>{/each}
          </select>
        </div>
        <div class="field"><label>TRIGGER</label>
          <select name="trigger" required>{#each TRIGGERS as t}<option value={t}>{t}</option>{/each}</select>
        </div>
        <div class="field"><label>MIN LEVEL</label><input name="minLevel" type="number" min="0" max="100"></div>
        <div class="field"><label>ITEM</label>
          <select name="itemId"><option value="">— none —</option>{#each data.allItems as i}<option value={i.id}>{i.name}</option>{/each}</select>
        </div>
        <div class="field"><label>MIN HAPPINESS</label><input name="minHappiness" type="number" min="0" max="255"></div>
        <div class="field"><label>TIME OF DAY</label>
          <select name="timeOfDay"><option value="">— any —</option>{#each ['day','night','dusk'] as t}<option value={t}>{t}</option>{/each}</select>
        </div>
        <button type="submit" class="save-btn-sm">SET PREDECESSOR</button>
      </form>
    </details>
    {/if}

    <div class="evo-tree">
      {#each evoTreeData().roots as root (root.id)}
        {@render evoNode(root.id)}
      {/each}
    </div>

    <details class="evo-add-succ">
      <summary>+ ADD SUCCESSOR</summary>
      <form action="?/addSuccessor" method="post" use:enhance class="evo-inline-form">
        <div class="field full"><label>CHILD POKÉMON</label>
          <select name="childPokemonId" required>
            <option value="">— pick —</option>
            {#each data.allPokemon as pk}<option value={pk.id}>{pk.name} ({pk.slug})</option>{/each}
          </select>
        </div>
        <button type="submit" class="save-btn-sm">ADD</button>
      </form>
    </details>

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
        <RetroSelect
          value={data.vgCounts.find(v => v.id === data.selectedVgId)?.slug}
          options={data.vgCounts.map(v => ({ value: v.slug, label: `${v.slug.toUpperCase()} (${v.count})` }))}
          onchange={onVgChange}
          variant="editor"
        />
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
          <RetroSelect
            name="sourceVgId"
            bind:value={copySourceVgId}
            options={[
              { value: '', label: '— PICK SOURCE VG —' },
              ...data.vgCounts.filter(v => v.id !== data.selectedVgId && v.count > 0).map(v => ({ value: v.id, label: `${v.slug.toUpperCase()} (${v.count})` }))
            ]}
            variant="editor"
          />
          <button type="submit" class="save-btn-sm">COPY</button>
        </form>
      </details>
    </div>
    {#if data.selectedVgId == null}
      <p class="evo-empty">No version groups with data. Add an entry below to start one.</p>
    {/if}
    {#if data.learnset.length > 0}
      <div class="learnset-table">
        <div class="learnset-head"><span>METHOD</span><span>MOVE</span><span>LEVEL</span><span class="text-right">REMOVE</span></div>
        {#each data.learnset as entry (`${entry.moveId}-${entry.learnMethodId}-${entry.level}`)}
          <form action="?/removeLearnsetEntry" method="post" use:enhance class="learnset-row">
            <input type="hidden" name="moveId" value={entry.moveId}>
            <input type="hidden" name="learnMethodId" value={entry.learnMethodId}>
            <input type="hidden" name="level" value={entry.level ?? 0}>
            <input type="hidden" name="versionGroupId" value={data.selectedVgId}>
            <span class="learnset-method">{entry.learnMethodName.toUpperCase()}</span>
            <div class="learnset-move-cell">
              <a href="/wiki/moves/{entry.moveSlug}" class="learnset-move">{entry.moveName.toUpperCase().replace(/-/g, ' ')}</a>
              <div class="learnset-move-badges">
                {#if entry.damageClass}
                  <MoveCategoryIcon category={entry.damageClass} size="sm" />
                {/if}
                <TypeBadge name={entry.typeName} hexColor={entry.typeHexColor} size="xs" />
              </div>
            </div>
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
        <div class="field">
          <label>METHOD</label>
          <RetroSelect
            name="learnMethodId"
            value={data.allLearnMethods.find(m => m.slug === 'level-up')?.id}
            options={data.allLearnMethods.map(m => ({ value: m.id, label: m.name.toUpperCase() }))}
            variant="editor"
          />
        </div>
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
    text-transform: uppercase;
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
    background-color: var(--theme-bg-tertiary);
    box-shadow: var(--shadow-card);
    margin-top: -4px;
  }
  .meta-field { display: flex; align-items: center; gap: var(--spacing-2); }
  .meta-label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-inverse); opacity: 0.85; white-space: nowrap; }
  .meta-input, .meta-select {
    background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 11px;
    padding: 2px var(--spacing-2);
  }
  .meta-input { width: 120px; }
  .wiki-link { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); text-decoration: none; }
  .wiki-link:hover { text-decoration: underline; }
  .meta-actions { display: flex; align-items: center; gap: var(--spacing-3); margin-left: auto; }



  /* Shared button styles */
  .save-btn {
    font-family: var(--font-display); font-size: 10px; color: var(--theme-text-primary);
    background-color: var(--theme-bg-secondary); border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2) var(--spacing-4); cursor: pointer;
    box-shadow: var(--shadow-card-subtle); transition: all 0.1s ease;
  }
  .save-btn:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }
  .save-btn:active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }

  .save-btn-sm {
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-primary);
    background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    padding: 2px var(--spacing-3); cursor: pointer; transition: all 0.1s ease;
    box-shadow: var(--shadow-card-subtle);
  }
  .save-btn-sm:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 2px 2px 0 var(--theme-text-accent);
  }
  .save-btn-sm:active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }

  .cancel-link {
    font-family: var(--font-display); font-size: 9px; color: var(--theme-text-inverse);
    opacity: 0.85; text-decoration: none; transition: all 0.1s ease;
  }
  .cancel-link:hover {
    color: var(--theme-text-accent);
    opacity: 1;
    transform: translate(-1px, -1px);
  }
  .cancel-link:active {
    transform: translate(1px, 1px);
  }
  .save-ok { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-accent); }
  .form-error { font-family: var(--font-mono); font-size: 10px; color: var(--color-text-error); }
  .delete-btn-sm {
    font-family: var(--font-display); font-size: 9px; color: var(--color-text-error);
    background: none; border: 2px solid var(--color-text-error);
    padding: 2px var(--spacing-3); cursor: pointer;
  }

  /* Content grid - identical to wiki */
  .content-grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-6);
  }
  .sprite-cell {
    display: flex;
    flex-direction: column;
  }
  .sprite-cell :global(.sprite-viewer) {
    --image-area-height: 160px;
  }
  .stats-cell {
    display: flex;
    flex-direction: column;
  }
  .stats-cell :global(.info-box) {
    height: 100%;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
  }
  .stats-cell :global(.stats-container) {
    display: flex;
    flex-direction: column;
    flex: 1;
  }
  .stats-cell :global(.stat-total) {
    margin-top: auto;
  }
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
    border-top: 2px dashed var(--theme-border-primary);
    background-color: var(--theme-bg-primary);
  }
  .sprite-field { display: flex; align-items: center; gap: var(--spacing-2); }
  .sprite-label { font-family: var(--font-display); font-size: 8px; color: var(--theme-text-inverse); opacity: 0.8; width: 60px; flex-shrink: 0; }
  .sprite-url-input {
    flex: 1; background-color: var(--theme-bg-secondary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-secondary); font-family: var(--font-mono); font-size: 10px;
    padding: 2px var(--spacing-2);
  }

  .stats-container { display: flex; flex-direction: column; gap: var(--spacing-2); }
  .stat-total {
    display: flex;
    justify-content: space-between;
    margin-top: auto;
    padding: var(--spacing-3) var(--spacing-4);
    margin-left: calc(-1 * var(--spacing-4));
    margin-right: calc(-1 * var(--spacing-4));
    margin-bottom: calc(-1 * var(--spacing-4));
    border-top: 2px dashed var(--theme-border-primary);
    background-color: var(--theme-bg-tertiary);
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-inverse);
  }

  .save-btn-sm {
    font-family: var(--font-display); font-size: 10px;
    padding: 2px var(--spacing-3);
    background: none; border: 2px solid var(--theme-text-accent); color: var(--theme-text-accent);
    cursor: pointer;
    transition: all 0.1s ease;
  }
  .save-btn-sm:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 2px 2px 0 var(--theme-text-accent);
  }
  .save-btn-sm:active {
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }



  /* Sub-forms inside InfoBox (types, abilities) */
  .sub-form-inline { display: flex; flex-direction: column; gap: var(--spacing-3); }
  .sub-field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .sub-label { font-family: var(--font-display); font-size: 8px; color: var(--theme-text-muted); }
  .sub-field select {
    background-color: var(--theme-bg-primary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 11px;
    padding: 2px var(--spacing-2);
  }


  /* Tab sections */
  .tab-section { display: flex; flex-direction: column; gap: var(--spacing-4); }
  .section-title { font-family: var(--font-display); font-size: 12px; color: var(--theme-text-primary); margin: 0; border-bottom: 2px solid var(--theme-border-primary); padding-bottom: var(--spacing-2); }
  /* Visual evo tree */
  .evo-tree { display: flex; flex-direction: column; align-items: center; overflow-x: auto; padding-bottom: var(--spacing-2); }
  .evo-node-wrap { display: flex; flex-direction: column; align-items: center; }
  .evo-conn-single {
    display: flex; flex-direction: column; align-items: center; gap: var(--spacing-1);
    padding: var(--spacing-3) 0 var(--spacing-2); position: relative; z-index: 0;
  }
  .evo-conn-single::before {
    content: ''; position: absolute; top: 0; bottom: 0; left: 50%; transform: translateX(-50%);
    width: 2px; background: var(--theme-border-primary); z-index: -1;
  }
  .evo-stem { width: 2px; height: var(--spacing-4); background: var(--theme-border-primary); flex-shrink: 0; }
  .evo-chunks-wrap { display: flex; flex-direction: column; align-items: center; position: relative; }
  .evo-branch-row { display: flex; flex-direction: row; align-items: flex-start; gap: var(--spacing-4); }
  .evo-branch-child { display: flex; flex-direction: column; align-items: center; position: relative; overflow: visible; }
  .evo-branch-child::after {
    content: ''; position: absolute; top: 0; left: 50%; transform: translateX(-50%);
    width: 2px; height: var(--spacing-4); background: var(--theme-border-primary);
  }
  .evo-branch-child.first-branch::before {
    content: ''; position: absolute; top: 0; left: 50%; right: calc(-1 * var(--spacing-4) / 2);
    height: 2px; background: var(--theme-border-primary);
  }
  .evo-branch-child.last-branch::before {
    content: ''; position: absolute; top: 0; right: 50%; left: calc(-1 * var(--spacing-4) / 2);
    height: 2px; background: var(--theme-border-primary);
  }
  .evo-branch-child.mid-branch::before {
    content: ''; position: absolute; top: 0; left: calc(-1 * var(--spacing-4) / 2); right: calc(-1 * var(--spacing-4) / 2);
    height: 2px; background: var(--theme-border-primary);
  }
  .evo-conn-cell {
    display: flex; flex-direction: column; align-items: center; gap: var(--spacing-1);
    padding: var(--spacing-4) 0 var(--spacing-2); position: relative; z-index: 1;
  }
  .evo-method-tag {
    font-family: var(--font-display); font-size: 8px; color: var(--theme-text-secondary);
    background: var(--theme-bg-secondary); padding: 3px 6px; text-align: center;
    white-space: nowrap; position: relative; z-index: 1;
  }
  .evo-down-arrow { font-size: 18px; color: var(--theme-text-secondary); line-height: 1; position: relative; z-index: 1; }
  /* Connector edit controls */
  .evo-conn-actions { display: flex; gap: var(--spacing-2); align-items: center; margin-top: var(--spacing-1); }
  .evo-conn-edit > summary.evo-edit-btn {
    font-family: var(--font-display); font-size: 10px; cursor: pointer;
    padding: 2px 6px; border: 1px solid var(--theme-text-muted); color: var(--theme-text-muted);
    list-style: none;
  }
  .evo-conn-edit > summary.evo-edit-btn:hover { border-color: var(--theme-text-accent); color: var(--theme-text-accent); }
  .evo-unlink-btn {
    font-family: var(--font-display); font-size: 12px; line-height: 1;
    padding: 1px 5px; background: none; border: 1px solid var(--color-text-error);
    color: var(--color-text-error); cursor: pointer;
  }
  .evo-unlink-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }

  /* Inline forms and set-predecessor / add-successor toggles */
  .evo-inline-form {
    display: flex; flex-wrap: wrap; gap: var(--spacing-2); align-items: flex-end;
    padding: var(--spacing-3); background: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary); margin-top: var(--spacing-2);
  }
  .evo-add-pred, .evo-add-succ { margin: var(--spacing-3) auto; text-align: center; }
  .evo-add-pred > summary, .evo-add-succ > summary {
    font-family: var(--font-display); font-size: 10px; cursor: pointer;
    color: var(--theme-text-accent); list-style: none;
  }

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


  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input[type=text], .field input[type=number], .field select {
    background-color: var(--theme-bg-secondary); border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary); font-family: var(--font-mono); font-size: 11px;
    padding: var(--spacing-1) var(--spacing-2);
  }
  .field.full { grid-column: 1 / -1; }

  .add-toggle { margin-top: var(--spacing-2); }
  .add-toggle summary { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-accent); cursor: pointer; }
  .add-form { margin-top: var(--spacing-3); }

  /* Learnset styles */
  .learnset-controls { display: flex; align-items: center; gap: var(--spacing-4); flex-wrap: wrap; }
  .learnset-table { display: flex; flex-direction: column; }
  .learnset-head { display: grid; grid-template-columns: 120px 1fr 60px 80px; gap: var(--spacing-3); padding: var(--spacing-2) var(--spacing-3); background-color: var(--theme-bg-primary); border-bottom: 2px solid var(--theme-border-primary); font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .learnset-row { display: grid; grid-template-columns: 120px 1fr 60px 80px; gap: var(--spacing-3); padding: var(--spacing-2) var(--spacing-3); border-bottom: 1px solid var(--theme-border-primary); align-items: center; }
  .learnset-method { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
  .learnset-move-cell { display: flex; align-items: center; justify-content: space-between; gap: var(--spacing-3); flex: 1; }
  .learnset-move-badges { display: flex; align-items: center; gap: var(--spacing-2); }
  .learnset-move { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-accent); text-decoration: none; font-weight: bold; }
  .learnset-level { font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-secondary); }
  .text-right { text-align: right; }
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

  .sprite-input-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    width: 100%;
  }
  .retro-upload-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 8px;
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
    padding: 4px 10px;
    cursor: pointer;
    white-space: nowrap;
    text-transform: uppercase;
    transition: all 0.05s ease;
  }
  .retro-upload-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0 var(--theme-border-primary);
  }
  .retro-upload-btn.selected {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }
  .sprite-section-divider {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-muted);
    border-top: 2px dashed var(--theme-border-primary);
    margin: var(--spacing-2) 0;
    padding-top: var(--spacing-3);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-align: center;
  }
  .hidden-file-input {
    display: none !important;
  }

  /* Dex nav + version selector side by side */
  .meta-row {
    display: flex;
    gap: var(--spacing-4);
    align-items: stretch;
    margin-bottom: var(--spacing-4);
  }

  .meta-row > :global(.version-selector-container) {
    flex: 1;
    min-width: 0;
  }

  /* Dex navigation strip */
  .dex-nav {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    flex-wrap: wrap;
    flex: 1;
    min-width: 0;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
  }

  :global(.dex-select) {
    min-width: 160px;
  }

  .nav-center {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    flex: 1;
    justify-content: center;
  }

  .nav-link {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    text-decoration: none;
    padding: 4px 8px;
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    transition: all 0.05s ease;
  }

  .nav-link:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0px var(--theme-text-accent);
    transform: translate(-1px, -1px);
  }

  .nav-entry {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .nav-disabled {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    opacity: 0.3;
    padding: 4px 8px;
  }

  .nav-text {
    display: inline;
  }

  @media (max-width: 600px) {
    .nav-text {
      display: none;
    }
    .dex-nav {
      flex-wrap: nowrap;
      justify-content: space-between;
      gap: var(--spacing-2);
      padding: var(--spacing-2) var(--spacing-3);
    }
    .nav-center {
      gap: var(--spacing-1);
    }
  }

  /* Flavor text editor */
  .flavor-text-editor-section {
    margin-bottom: var(--spacing-6);
  }
</style>
