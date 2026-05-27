<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import StatBar from '$lib/components/StatBar.svelte';
  import MoveCard from '$lib/components/MoveCard.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import TypeMatchupDisplay from '$lib/components/TypeMatchupDisplay.svelte';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import Tabs from '$lib/components/Tabs.svelte';
  import PokemonSpriteViewer from '$lib/components/PokemonSpriteViewer.svelte';
  import PokemonDataList from '$lib/components/PokemonDataList.svelte';
  import EvolutionChain from '$lib/components/EvolutionChain.svelte';
  import { goto, afterNavigate } from '$app/navigation';
  import { onMount } from 'svelte';
  import { selectedPokedexSlug } from '$lib/stores/selectedPokedex';

  let { data } = $props();

  let p = $derived(data.pokemon);
  let s = $derived(data.species);



  let totalStats = $derived(data.stats.reduce((acc, stat) => acc + stat.baseStat, 0));

  // Gender display
  let genderText = $derived(() => {
    const gr = s.genderRate;
    if (gr === null) return 'UNKNOWN';
    if (gr === -1)   return 'GENDERLESS';
    if (gr === 0)    return '100% ♂';
    if (gr === 8)    return '100% ♀';
    const femaleChance = Math.round((gr / 8) * 100);
    return `${100 - femaleChance}% ♂ / ${femaleChance}% ♀`;
  });

  // Version Mechanics
  let versionIdsWithData = $derived(new Set([
    ...data.flavorTexts.map(f => f.versionGroupId),
    ...data.learnset.map(l => l.versionGroupId)
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

  let currentFlavorText = $derived(() => {
    if (!selectedVersionId) return 'No Pokédex entry available.';
    const entry = data.flavorTexts.find(f => f.versionGroupId === selectedVersionId);
    return entry ? entry.flavorText.replace(/\f|\n/g, ' ') : 'No Pokédex entry available for this version.';
  });

  let currentLearnset = $derived(() => {
    if (!selectedVersionId) return [];
    return data.learnset.filter(l => l.versionGroupId === selectedVersionId);
  });

  // Learnset tabs
  const methodLabels: Record<string, string> = {
    'level-up': 'LEVEL UP',
    'machine':  'TM / HM',
    'egg':      'EGG MOVES',
    'tutor':    'MOVE TUTOR',
  };

  function methodLabel(name: string): string {
    return methodLabels[name] ?? name.toUpperCase();
  }

  let learnsetMethods = $derived(() => {
    const seen = new Set<string>();
    const order = ['level-up', 'machine', 'egg', 'tutor'];
    const methods = currentLearnset()
      .map(l => l.methodName)
      .filter(m => { if (seen.has(m)) return false; seen.add(m); return true; });
    methods.sort((a, b) => {
      const ai = order.indexOf(a); const bi = order.indexOf(b);
      if (ai === -1 && bi === -1) return a.localeCompare(b);
      if (ai === -1) return 1;
      if (bi === -1) return -1;
      return ai - bi;
    });
    return methods;
  });

  let activeMethod = $state('');

  const PAGE_TABS = [
    { key: 'info',     label: 'INFO' },
    { key: 'forms',    label: 'FORMS' },
    { key: 'learnset', label: 'LEARNSET' },
    { key: 'location', label: 'LOCATION' },
  ] as const;

  let activePageTab = $state('info');

  afterNavigate(() => {
    activePageTab = 'info';
    activeChainTab = data.formSuffix ? data.formSuffix.toUpperCase() : 'EVOLUTION';
  });

  $effect(() => {
    const methods = learnsetMethods();
    if (methods.length > 0 && !methods.includes(activeMethod)) {
      activeMethod = methods.includes('level-up') ? 'level-up' : methods[0];
    }
  });

  let learnsetTabs = $derived(() =>
    learnsetMethods().map(m => ({ key: m, label: methodLabel(m) }))
  );

  let activeLearnset = $derived(() => {
    if (!activeMethod) return [];
    const moves = currentLearnset().filter(l => l.methodName === activeMethod);
    if (activeMethod === 'level-up') {
      return moves.sort((a, b) => (a.level ?? 0) - (b.level ?? 0));
    }
    return moves.sort((a, b) => a.moveName.localeCompare(b.moveName));
  });

  // Location encounters
  type LocationEncounterRow = typeof data.locationEncounters[0];

  type LocationGroup = {
    name: string;
    slug: string;
    areas: { areaName: string; encounters: LocationEncounterRow[] }[];
  };

  let currentLocationEncounters = $derived(() => {
    if (!selectedVersionId) return [];
    return data.locationEncounters.filter(e => e.versionGroupId === selectedVersionId);
  });

  let locationGroups = $derived((): LocationGroup[] => {
    const groups = new Map<string, LocationGroup>();
    for (const enc of currentLocationEncounters()) {
      if (!groups.has(enc.locationSlug)) {
        groups.set(enc.locationSlug, { name: enc.locationName, slug: enc.locationSlug, areas: [] });
      }
      const loc = groups.get(enc.locationSlug)!;
      let area = loc.areas.find(a => a.areaName === enc.areaName);
      if (!area) { area = { areaName: enc.areaName, encounters: [] }; loc.areas.push(area); }
      area.encounters.push(enc);
    }
    return [...groups.values()];
  });

  function moveMethod(move: typeof data.learnset[0]): string | undefined {
    if (move.methodName === 'level-up') {
      return `Lv.${(move.level ?? 0).toString().padStart(3, ' ')}`;
    }
    return undefined;
  }

  // Chain tab system
  type ChainForm = typeof data.chainSpecies[0]['forms'][0];
  type ChainSpecies = typeof data.chainSpecies[0];
  type ChainTab =
    | { kind: 'default';   label: string }
    | { kind: 'regional';  label: string; suffix: string }
    | { kind: 'alternate'; label: string; forms: ChainForm[] }
    | { kind: 'gmax';      label: string; forms: ChainForm[] }
    | { kind: 'mega';      label: string; forms: ChainForm[] };

  const REGIONAL_SUFFIXES_CHAIN = ['alola', 'galar', 'hisui', 'paldea'];

  let chainTabs = $derived((): ChainTab[] => {
    const tabs: ChainTab[] = [{ kind: 'default', label: 'EVOLUTION' }];
    const seenSuffixes = new Set<string>();
    const gmaxForms: ChainForm[] = [];
    const megaForms: ChainForm[] = [];
    const otherAltForms: ChainForm[] = [];

    function bucketAlt(form: ChainForm) {
      if (form.slug.includes('-gmax')) {
        if (!gmaxForms.some(f => f.slug === form.slug)) gmaxForms.push(form);
      } else if (form.slug.includes('-mega')) {
        if (!megaForms.some(f => f.slug === form.slug)) megaForms.push(form);
      } else {
        if (!otherAltForms.some(f => f.slug === form.slug)) otherAltForms.push(form);
      }
    }

    for (const sp of data.chainSpecies) {
      for (const form of sp.forms) {
        if (form.isDefault) continue;
        const suffix = REGIONAL_SUFFIXES_CHAIN.find(r => form.slug.includes('-' + r));
        if (suffix) {
          const afterSuffix = form.slug.slice(form.slug.indexOf('-' + suffix) + 1 + suffix.length);
          if (afterSuffix.length > 0) {
            // Sub-regional form (e.g. tauros-paldea-combat) — treat as alt form
            bucketAlt(form);
          } else if (!seenSuffixes.has(suffix)) {
            seenSuffixes.add(suffix);
            tabs.push({ kind: 'regional', label: suffix.toUpperCase(), suffix });
          }
        } else {
          bucketAlt(form);
        }
      }
    }

    if (megaForms.length > 0)     tabs.push({ kind: 'mega',      label: 'MEGA',  forms: megaForms });
    if (gmaxForms.length > 0)     tabs.push({ kind: 'gmax',      label: 'GMAX',  forms: gmaxForms });
    if (otherAltForms.length > 0) tabs.push({ kind: 'alternate', label: 'FORMS', forms: otherAltForms });
    return tabs;
  });

  let activeChainTab = $state<string>('EVOLUTION');
  $effect(() => {
    activeChainTab = data.formSuffix ? data.formSuffix.toUpperCase() : 'EVOLUTION';
  });

  let activeTab = $derived(chainTabs().find(t => t.label === activeChainTab) ?? chainTabs()[0]);

  let chainTabItems = $derived(() =>
    chainTabs().map(t => ({ key: t.label, label: t.label }))
  );



  // On mount: if no ?dex= param set, redirect using the stored slug (if valid for this Pokémon)
  onMount(() => {
    if (data.nav && !new URL(window.location.href).searchParams.has('dex')) {
      const storedSlug = $selectedPokedexSlug;
      if (storedSlug && data.nav.dexList.find((d: any) => d.slug === storedSlug)) {
        goto(`?dex=${storedSlug}`, { replaceState: true });
      }
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement).tagName;
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return;
    if (!data.nav) return;
    if (e.key === 'ArrowLeft' && data.nav.prev) {
      goto(`/wiki/pokemon/${data.nav.prev.pokemonSlug}?dex=${data.nav.selectedDex.slug}`);
    } else if (e.key === 'ArrowRight' && data.nav.next) {
      goto(`/wiki/pokemon/${data.nav.next.pokemonSlug}?dex=${data.nav.selectedDex.slug}`);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <title>{p.name.toUpperCase()} | Karta Yaxu</title>
</svelte:head>

<div class="pokemon-detail">
  <!-- Header -->
  <header class="detail-header">
    <div class="dex-num">#{(p.pokeapiId ?? 0).toString().padStart(3, '0')}</div>
    <div class="header-main">
      <h1>{p.name.toUpperCase()}</h1>
      {#if data.genus}
        <div class="genus">{data.genus}</div>
      {/if}
    </div>
    <div class="types">
      {#each data.types as t}
        <TypeBadge name={t.name} hexColor={t.hexColor} size="lg" />
      {/each}
    </div>
  </header>

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
            href="/wiki/pokemon/{data.nav.prev.pokemonSlug}?dex={data.nav.selectedDex.slug}"
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
            onchange={(val) => { selectedPokedexSlug.set(val); goto(`?dex=${val}`); }}
            class="dex-select"
          />
        </div>

        {#if data.nav.next}
          <a
            href="/wiki/pokemon/{data.nav.next.pokemonSlug}?dex={data.nav.selectedDex.slug}"
            class="nav-link nav-next"
          ><span class="nav-text">{data.nav.next.speciesName.toUpperCase()}</span> →</a>
        {:else}
          <span class="nav-disabled">→</span>
        {/if}
      </nav>
    {/if}
  </div>

  <Tabs tabs={PAGE_TABS} bind:active={activePageTab}>
  {#if activePageTab === 'info'}
  <div class="content-grid">
    <!-- Top-left: Sprite only -->
    <div class="sprite-cell">
      <PokemonSpriteViewer
        spriteDefault={p.spriteDefault}
        spriteShiny={p.spriteShiny}
        spriteArtwork={p.spriteArtwork}
        spriteArtworkShiny={p.spriteArtworkShiny}
        spriteBackDefault={p.spriteBackDefault}
        spriteBackShiny={p.spriteBackShiny}
        pokeapiId={p.pokeapiId}
        name={p.name}
      />
    </div>

    <!-- Top-right: Base Stats -->
    <div class="stats-cell">
      <InfoBox title="BASE STATS">
        <div class="stats-container">
          {#each data.stats as stat}
            <StatBar name={stat.statName} value={stat.baseStat} max={255} />
          {/each}

          <div class="stat-total">
            <span class="label">TOTAL:</span>
            <span class="value">{totalStats}</span>
          </div>
        </div>
      </InfoBox>
    </div>

    <!-- Bottom-left: Version Selector + Flavor Text + Abilities -->
    <div class="info-cell">
      {#if validVersions.length > 0}
        <FlavorTextBox text={currentFlavorText()} />
      {/if}

      <InfoBox title="ABILITIES">
        <ul class="ability-list">
          {#each data.abilities as a}
            <li class:hidden-ability={a.isHidden}>
              <a href="/wiki/abilities/{a.slug}" class="ability-link">
                <span class="name">{a.name.toUpperCase()}</span>
                {#if a.isHidden}
                  <span class="badge">HIDDEN</span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      </InfoBox>
    </div>

    <!-- Bottom-right: Data -->
    <div class="data-cell">
      <InfoBox title="DATA">
        <PokemonDataList
          height={p.height}
          weight={p.weight}
          captureRate={s.captureRate}
          genderText={genderText()}
          eggGroups={data.eggGroups.map(e => e.name.toUpperCase()).join(', ')}
          habitat={s.habitat}
          growthRate={s.growthRate}
          happiness={s.baseHappiness}
        />
      </InfoBox>
    </div>
  </div>

  {#if data.typeMatchup.length > 0 || data.offensiveMatchup.length > 0}
    <TypeMatchupDisplay offensive={data.offensiveMatchup} defensive={data.typeMatchup} />
  {/if}

  {:else if activePageTab === 'forms'}
  <!-- Evolution Chain -->
  <Tabs tabs={chainTabItems()} bind:active={activeChainTab}>
    <EvolutionChain
      chainSpecies={data.chainSpecies}
      evolutionChain={data.evolutionChain}
      activePokemon={data.pokemon}
      activeTab={activeTab}
      variant="wiki"
    />
  </Tabs>

  {:else if activePageTab === 'learnset'}
  <!-- Learnset -->
  {#if validVersions.length > 0}
    <Tabs tabs={learnsetTabs()} bind:active={activeMethod}>
      {#if activeLearnset().length > 0}
        <div class="move-list">
          {#each activeLearnset() as move (move.moveSlug + move.methodName + move.level)}
            <MoveCard
              name={move.moveName}
              slug={move.moveSlug}
              typeName={move.typeName}
              typeHexColor={move.typeHexColor}
              damageClass={move.damageClass}
              power={move.power}
              accuracy={move.accuracy}
              pp={move.pp}
              method={moveMethod(move)}
            />
          {/each}
        </div>
      {:else}
        <p class="empty-text">No moves learned in this version.</p>
      {/if}
    </Tabs>
  {:else}
    <p class="empty-text">No learnset data available.</p>
  {/if}

  {:else if activePageTab === 'location'}
  {#if locationGroups().length === 0}
    <p class="empty-text">No encounter data for this version.</p>
  {:else}
    <div class="location-encounter-list">
      {#each locationGroups() as loc}
        <div class="location-section">
          <a href="/wiki/locations/{loc.slug}" class="location-heading">
            {loc.name.replace(/-/g, ' ').toUpperCase()}
          </a>
          {#each loc.areas as area}
            {#each area.encounters as enc}
              <div class="enc-row">
                <span class="enc-method">{enc.methodName.replace(/-/g, ' ').toUpperCase()}</span>
                <span class="enc-level">LV.{enc.minLevel === enc.maxLevel ? enc.minLevel : `${enc.minLevel}–${enc.maxLevel}`}</span>
                <span class="enc-chance">{enc.chance}%</span>
              </div>
            {/each}
          {/each}
        </div>
      {/each}
    </div>
  {/if}
  {/if}
</Tabs>
</div>

<style>
  .pokemon-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  /* ── Header ─────────────────────────────────────────────────────────────── */

  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: var(--shadow-card);
    flex-wrap: wrap;
  }

  .dex-num {
    font-family: var(--font-display);
    font-size: 16px;
    color: var(--theme-text-secondary);
    flex-shrink: 0;
  }

  .header-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .genus {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
  }

  .types {
    display: flex;
    gap: var(--spacing-2);
  }

  /* ── Layout ──────────────────────────────────────────────────────────────── */

  .content-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
  }

  @media (max-width: 992px) {
    .pokemon-detail { gap: var(--spacing-3); }

    .meta-row { flex-direction: column; }

    .detail-header {
      padding: var(--spacing-2) var(--spacing-3);
      flex-wrap: wrap;
      gap: var(--spacing-2);
    }

    h1 { font-size: 16px; }

    .genus { font-size: 10px; }

    .content-grid {
      grid-template-columns: minmax(0, 1fr);
      gap: var(--spacing-3);
    }

    .sprite-cell,
    .stats-cell,
    .info-cell,
    .data-cell {
      min-width: 0;
    }

    .image-box {
      min-height: 160px;
      max-height: 220px;
      padding: var(--spacing-2);
      overflow: hidden;
    }





    .ability-list { gap: var(--spacing-2); }

    .ability-link { font-size: 11px; }
  }

  .sprite-cell {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
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

  .info-cell {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .data-cell {
    display: flex;
    flex-direction: column;
  }

  /* ── Shared Box ──────────────────────────────────────────────────────────── */



  /* ── Image ───────────────────────────────────────────────────────────────── */





  /* ── Abilities ───────────────────────────────────────────────────────────── */

  .ability-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .ability-list li { display: flex; align-items: center; }

  .ability-list li.hidden-ability .ability-link {
    color: var(--theme-text-secondary);
  }

  .ability-link {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-2);
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
    text-decoration: none;
    border: 3px solid transparent;
    padding: 2px 6px;
    margin: -2px -6px;
    transition: all 0.05s ease;
  }

  .ability-link:hover {
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0px var(--theme-text-accent);
    transform: translate(-2px, -2px);
    color: var(--theme-text-accent);
  }

  .ability-list .badge {
    font-family: var(--font-display);
    font-size: 8px;
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    padding: 2px 4px;
  }

  /* ── Stats ───────────────────────────────────────────────────────────────── */

  .stats-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

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

  /* ── Learnset ────────────────────────────────────────────────────────────── */

  .move-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .empty-text {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }

  /* Dex nav + version selector side by side */
  .meta-row {
    display: flex;
    gap: var(--spacing-4);
    align-items: stretch;
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

  /* ── Location encounters ─────────────────────────────────────────────────── */

  .location-encounter-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
  }

  .location-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    box-shadow: var(--shadow-card);
  }

  .location-heading {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    text-decoration: none;
    border-bottom: 2px solid var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
    line-height: 1.4;
    transition: color 0.05s ease;
  }

  .location-heading:hover { color: var(--theme-text-accent); }

  .enc-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .enc-method {
    flex: 1;
    color: var(--theme-text-secondary);
    font-family: var(--font-display);
    font-size: 9px;
  }

  .enc-level {
    color: var(--theme-text-primary);
    font-size: 10px;
    white-space: nowrap;
  }

  .enc-chance {
    color: var(--theme-text-muted);
    font-size: 10px;
    white-space: nowrap;
    min-width: 28px;
    text-align: right;
  }
</style>
