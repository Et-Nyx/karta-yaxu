<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import StatBar from '$lib/components/StatBar.svelte';
  import MoveCard from '$lib/components/MoveCard.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import TypeMatchupDisplay from '$lib/components/TypeMatchupDisplay.svelte';

  let { data } = $props();

  let p = $derived(data.pokemon);
  let s = $derived(data.species);

  let isShiny = $state(false);

  let spriteUrl = $derived(
    isShiny
      ? (p.pokeapiId
          ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/shiny/${p.pokeapiId}.png`
          : p.spriteShiny)
      : (p.spriteArtwork ||
          p.spriteDefault ||
          (p.pokeapiId
            ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/${p.pokeapiId}.png`
            : null))
  );

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
  const versionIdsWithData = new Set([
    ...data.flavorTexts.map(f => f.versionGroupId),
    ...data.learnset.map(l => l.versionGroupId)
  ]);

  let validVersions = $derived(data.availableVersions.filter(v => versionIdsWithData.has(v.id)));
  let selectedVersionId = $state<number | null>(null);

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

  let activeMethod = $state<string | null>(null);

  $effect(() => {
    const methods = learnsetMethods();
    if (methods.length > 0 && (activeMethod === null || !methods.includes(activeMethod))) {
      activeMethod = methods.includes('level-up') ? 'level-up' : methods[0];
    }
  });

  let activeLearnset = $derived(() => {
    if (!activeMethod) return [];
    const moves = currentLearnset().filter(l => l.methodName === activeMethod);
    if (activeMethod === 'level-up') {
      return moves.sort((a, b) => (a.level ?? 0) - (b.level ?? 0));
    }
    return moves.sort((a, b) => a.moveName.localeCompare(b.moveName));
  });

  function chunkArray<T>(arr: T[], size: number): T[][] {
    const chunks: T[][] = [];
    for (let i = 0; i < arr.length; i += size) chunks.push(arr.slice(i, i + size));
    return chunks;
  }

  function drawBranchLine(node: HTMLElement) {
    let svgEl: SVGSVGElement | null = null;

    function render() {
      svgEl?.remove();
      svgEl = null;

      const rows = Array.from(node.querySelectorAll<HTMLElement>(':scope > .evo-branch-row'));
      if (rows.length < 2) return;

      const color = getComputedStyle(node).getPropertyValue('--theme-border-primary').trim() || '#444';
      const cx = node.offsetWidth / 2;

      svgEl = document.createElementNS('http://www.w3.org/2000/svg', 'svg') as SVGSVGElement;
      svgEl.style.cssText = 'position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:0;overflow:visible';

      for (let i = 0; i < rows.length - 1; i++) {
        // y1: just below the 2px horizontal bar at the top of row i
        const y1 = rows[i].offsetTop + 2;
        // y2: top of the horizontal bar of row i+1
        const y2 = rows[i + 1].offsetTop;

        const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        line.setAttribute('x1', String(cx));
        line.setAttribute('y1', String(y1));
        line.setAttribute('x2', String(cx));
        line.setAttribute('y2', String(y2));
        line.setAttribute('stroke', color);
        line.setAttribute('stroke-width', '2');
        svgEl.appendChild(line);
      }

      node.appendChild(svgEl);
    }

    const ro = new ResizeObserver(() => requestAnimationFrame(render));
    ro.observe(node);
    requestAnimationFrame(() => requestAnimationFrame(render));

    return {
      destroy() {
        ro.disconnect();
        svgEl?.remove();
      }
    };
  }

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
    | { kind: 'alternate'; label: string; forms: ChainForm[] };

  const REGIONAL_SUFFIXES_CHAIN = ['alola', 'galar', 'hisui', 'paldea'];

  function primaryFormFor(sp: ChainSpecies, suffix: string | null): ChainForm {
    if (suffix) {
      const match = sp.forms.find(f => f.slug.includes('-' + suffix));
      if (match) return match;
    }
    return sp.forms.find(f => f.isDefault) ?? sp.forms[0];
  }

  let chainTabs = $derived((): ChainTab[] => {
    const tabs: ChainTab[] = [{ kind: 'default', label: 'DEFAULT' }];
    const rootIds = new Set(evoTree().roots);
    const seenSuffixes = new Set<string>();
    const altForms: ChainForm[] = [];

    for (const sp of data.chainSpecies) {
      for (const form of sp.forms) {
        if (form.isDefault) continue;
        const suffix = REGIONAL_SUFFIXES_CHAIN.find(r => form.slug.includes('-' + r));
        if (suffix) {
          const afterSuffix = form.slug.slice(form.slug.indexOf('-' + suffix) + 1 + suffix.length);
          if (afterSuffix.length > 0) {
            // Sub-regional form (e.g. tauros-paldea-combat) — treat as alt form so all variants show
            if (!altForms.some(f => f.slug === form.slug)) altForms.push(form);
          } else if (rootIds.has(sp.speciesId) && !seenSuffixes.has(suffix)) {
            seenSuffixes.add(suffix);
            tabs.push({ kind: 'regional', label: suffix.toUpperCase(), suffix });
          }
        } else {
          if (!altForms.some(f => f.slug === form.slug)) altForms.push(form);
        }
      }
    }

    if (altForms.length > 0) tabs.push({ kind: 'alternate', label: 'FORMS', forms: altForms });
    return tabs;
  });

  let activeChainTab = $state<string>(
    data.formSuffix ? data.formSuffix.toUpperCase() : 'DEFAULT'
  );

  let activeTab = $derived(chainTabs().find(t => t.label === activeChainTab) ?? chainTabs()[0]);

  // Evolution chain tree builder
  type EvoMethod = { text: string; itemSlug: string | null };

  function evoMethodLabel(link: typeof data.evolutionChain[0]): EvoMethod {
    switch (link.trigger) {
      case 'level-up':
        if (link.minHappiness) {
          const suffix = link.timeOfDay === 'day' ? ' ☀' : link.timeOfDay === 'night' ? ' ☾' : '';
          return { text: `FRIENDSHIP${suffix}`, itemSlug: null };
        }
        if (link.minLevel) return { text: `Lv. ${link.minLevel}`, itemSlug: null };
        return { text: 'LEVEL UP', itemSlug: null };
      case 'use-item':
        return {
          text: link.itemName?.toUpperCase().replace(/-/g, ' ') ?? 'USE ITEM',
          itemSlug: link.itemSlug ?? null,
        };
      case 'trade':
        return { text: 'TRADE', itemSlug: null };
      case 'shed':
        return { text: 'SHED', itemSlug: null };
      default:
        return { text: link.trigger.toUpperCase().replace(/-/g, ' '), itemSlug: null };
    }
  }

  let evoTree = $derived(() => {
    const links = data.evolutionChain;
    const speciesMap = new Map(data.chainSpecies.map(s => [s.speciesId, s]));
    type Child = { speciesId: number; method: EvoMethod; location: string | null; requiredSuffix: string | null };

    // Synthesize virtual edges for mid-chain species that have a regional form AND
    // have a form-restricted outgoing edge — so they appear as a separate branch.
    // Example: Mime Jr.→Mr.Mime (loc=null) + Mr.Mime has Galar form + Mr.Mime→Mr.Rime (req='galar')
    //          → synthesize Mime Jr.→Mr.Mime (loc='galar')
    const syntheticLinks = [...links];
    for (const edge of links) {
      if (edge.location !== null) continue;
      const toSp = speciesMap.get(edge.toSpeciesId);
      if (!toSp) continue;
      for (const suffix of REGIONAL_SUFFIXES_CHAIN) {
        const hasRegionalForm = toSp.forms.some(f => f.slug.includes('-' + suffix));
        if (!hasRegionalForm) continue;
        // If fromSpecies itself has this regional form, a regional tab already handles it
        const fromSp = speciesMap.get(edge.fromSpeciesId);
        if (fromSp?.forms.some(f => f.slug.includes('-' + suffix))) continue;
        const alreadyExists = links.some(
          e => e.fromSpeciesId === edge.fromSpeciesId
            && e.toSpeciesId === edge.toSpeciesId
            && e.location === suffix
        );
        if (!alreadyExists) syntheticLinks.push({ ...edge, location: suffix });
      }
    }

    // Dedup by (from:to:location) — preserves both Raichu edges AND synthesized Mr.Mime branches
    const seen = new Set<string>();
    const locationDeduped = syntheticLinks.filter(l => {
      const key = `${l.fromSpeciesId}:${l.toSpeciesId}:${l.location ?? ''}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });

    // For each (from, to) pair, if any edge is use-item, discard level-up edges for that pair.
    // This removes Leafeon/Glaceon level-up duplicates while keeping the Stone variants.
    const pairEdges = new Map<string, typeof locationDeduped>();
    for (const l of locationDeduped) {
      const key = `${l.fromSpeciesId}:${l.toSpeciesId}`;
      if (!pairEdges.has(key)) pairEdges.set(key, []);
      pairEdges.get(key)!.push(l);
    }
    const uniqueLinks = [...pairEdges.values()].flatMap(edges => {
      const hasItem = edges.some(e => e.trigger === 'use-item');
      return hasItem ? edges.filter(e => e.trigger === 'use-item') : edges;
    });

    const childrenOf = new Map<number, Child[]>();
    const allToIds = new Set<number>();
    for (const link of uniqueLinks) {
      allToIds.add(link.toSpeciesId);
      if (!childrenOf.has(link.fromSpeciesId)) childrenOf.set(link.fromSpeciesId, []);
      childrenOf.get(link.fromSpeciesId)!.push({
        speciesId: link.toSpeciesId,
        method: evoMethodLabel(link),
        location: link.location ?? null,
        requiredSuffix: link.requiredSuffix,
      });
    }

    const allFromIds = [...new Set([
      ...uniqueLinks.map(l => l.fromSpeciesId),
      ...speciesMap.keys(),
    ])];
    const roots = allFromIds.filter(id => !allToIds.has(id));
    return { roots, childrenOf, speciesMap };
  });
</script>

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
        <TypeBadge name={t.name} size="lg" />
      {/each}
    </div>
  </header>

  <div class="content-grid">
    <!-- Top-left: Sprite only -->
    <div class="sprite-cell">
      <div class="image-box">
        <img src={spriteUrl} alt={p.name} class:shiny-active={isShiny} style="image-rendering: pixelated;" />
        <button class="shiny-toggle" onclick={() => isShiny = !isShiny} title={isShiny ? 'Switch to Normal' : 'Switch to Shiny'}>
          {isShiny ? '◼ NORMAL' : '✦ SHINY'}
        </button>
      </div>
    </div>

    <!-- Top-right: Base Stats -->
    <div class="stats-cell">
      <div class="info-box stats-box">
        <h3>BASE STATS</h3>
        <div class="stats-container">
          {#each data.stats as stat}
            <StatBar name={stat.statName} value={stat.baseStat} max={255} />
          {/each}

          <div class="stat-total">
            <span class="label">TOTAL:</span>
            <span class="value">{totalStats}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Bottom-left: Version Selector + Flavor Text + Abilities -->
    <div class="info-cell">
      {#if validVersions.length > 0}
        <div class="version-selector-container">
          <label for="versionSelect">VERSION:</label>
          <select id="versionSelect" bind:value={selectedVersionId} class="version-select">
            {#each validVersions as v}
              <option value={v.id}>{v.name.toUpperCase().replace(/-/g, ' ')}</option>
            {/each}
          </select>
        </div>

        <div class="flavor-text-box">
          <p>{currentFlavorText()}</p>
        </div>
      {/if}

      <div class="info-box">
        <h3>ABILITIES</h3>
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
      </div>
    </div>

    <!-- Bottom-right: Data -->
    <div class="data-cell">
      <div class="info-box data-box">
        <h3>DATA</h3>
        <ul class="data-list">
          <li>
            <span class="label">HEIGHT</span>
            <span class="value">{p.height / 10} m</span>
          </li>
          <li>
            <span class="label">WEIGHT</span>
            <span class="value">{p.weight / 10} kg</span>
          </li>
          {#if s.captureRate !== null}
            <li>
              <span class="label">CAPTURE</span>
              <span class="value">{s.captureRate}</span>
            </li>
          {/if}
          {#if s.genderRate !== null}
            <li>
              <span class="label">GENDER</span>
              <span class="value">{genderText()}</span>
            </li>
          {/if}
          {#if data.eggGroups.length > 0}
            <li>
              <span class="label">EGG GROUP</span>
              <span class="value">{data.eggGroups.map(e => e.name.toUpperCase()).join(', ')}</span>
            </li>
          {/if}
          {#if s.habitat}
            <li>
              <span class="label">HABITAT</span>
              <span class="value">{s.habitat.toUpperCase()}</span>
            </li>
          {/if}
          {#if s.growthRate}
            <li>
              <span class="label">GROWTH</span>
              <span class="value">{s.growthRate.toUpperCase()}</span>
            </li>
          {/if}
          {#if s.baseHappiness !== null}
            <li>
              <span class="label">HAPPINESS</span>
              <span class="value">{s.baseHappiness}</span>
            </li>
          {/if}
        </ul>
      </div>
    </div>
  </div>

  {#if data.typeMatchup.length > 0 || data.offensiveMatchup.length > 0}
    <div class="type-matchup-section">
      <TypeMatchupDisplay offensive={data.offensiveMatchup} defensive={data.typeMatchup} />
    </div>
  {/if}

  <!-- Evolution Chain -->
  {#snippet evoNode(speciesId: number, tabSuffix: string | null, edgeLocation: string | null)}
    {@const effectiveSuffix = edgeLocation ?? tabSuffix}
    {@const allChildren = evoTree().childrenOf.get(speciesId) ?? []}
    {@const children = allChildren.filter(child =>
      child.requiredSuffix === effectiveSuffix ||
      (child.requiredSuffix === null && !allChildren.some(
        o => o.speciesId !== child.speciesId && o.requiredSuffix === effectiveSuffix
      ))
    )}
    {@const sp = evoTree().speciesMap.get(speciesId)}
    {#if sp}
      {@const pf = primaryFormFor(sp, effectiveSuffix)}
      <div class="evo-node-wrap">
        <div class="evo-card-cell" class:is-current={sp.speciesId === data.pokemon.speciesId}>
          <PokemonCard pokemon={{
            pokeapiId: pf.pokeapiId ?? 0,
            name: pf.name,
            slug: pf.slug,
            spriteUrl: pf.spriteUrl,
            types: pf.types,
            dexNumber: pf.dexNumber,
          }} />
        </div>

        {#if children.length === 1}
          <div class="evo-conn-single">
            {#if children[0].method.text}
              <span class="evo-method-tag">
                {#if children[0].method.itemSlug}
                  <a href="/wiki/items/{children[0].method.itemSlug}" class="evo-item-link">{children[0].method.text}</a>
                {:else}
                  {children[0].method.text}
                {/if}
              </span>
            {/if}
            <span class="evo-down-arrow">↓</span>
          </div>
          {@render evoNode(children[0].speciesId, tabSuffix, children[0].location)}
        {:else if children.length > 1}
          {@const chunks = chunkArray(children, 4)}
          <div class="evo-chunks-wrap" use:drawBranchLine>
            {#each chunks as chunk}
              <div class="evo-stem"></div>
              <div class="evo-branch-row">
                {#each chunk as child, i}
                  <div class="evo-branch-child"
                    class:first-branch={i === 0}
                    class:last-branch={i === chunk.length - 1}
                    class:mid-branch={i > 0 && i < chunk.length - 1}
                  >
                    <div class="evo-conn-cell">
                      {#if child.method.text}
                        <span class="evo-method-tag">
                          {#if child.method.itemSlug}
                            <a href="/wiki/items/{child.method.itemSlug}" class="evo-item-link">{child.method.text}</a>
                          {:else}
                            {child.method.text}
                          {/if}
                        </span>
                      {/if}
                      <span class="evo-down-arrow">↓</span>
                    </div>
                    {@render evoNode(child.speciesId, tabSuffix, child.location)}
                  </div>
                {/each}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  {/snippet}

  <div class="info-box evo-box">
      <h3>EVOLUTION CHAIN</h3>

      {#if chainTabs().length > 1}
        <div class="chain-tab-bar">
          {#each chainTabs() as tab}
            <button
              class="chain-tab-btn"
              class:active={activeChainTab === tab.label}
              onclick={() => activeChainTab = tab.label}
            >{tab.label}</button>
          {/each}
        </div>
      {/if}

      {#if activeTab?.kind === 'alternate'}
        <div class="alt-forms-grid">
          {#each activeTab.forms as form}
            <PokemonCard pokemon={{
              pokeapiId: form.pokeapiId ?? 0,
              name: form.name,
              slug: form.slug,
              spriteUrl: form.spriteUrl,
              types: form.types,
              dexNumber: form.dexNumber,
            }} />
          {/each}
        </div>
      {:else}
        <div class="evo-tree">
          {#each evoTree().roots as rootId}
            {@const activeSuffix = activeTab?.kind === 'regional' ? activeTab.suffix : null}
            {@render evoNode(rootId, activeSuffix, null)}
          {/each}
        </div>
      {/if}
    </div>

  <!-- Learnset -->
  {#if validVersions.length > 0}
    <div class="info-box learnset-box">
      <div class="learnset-header">
        <h3>LEARNSET</h3>
        <div class="version-label">
          {validVersions.find(v => v.id === selectedVersionId)?.name.toUpperCase().replace(/-/g, ' ')}
        </div>
      </div>

      {#if learnsetMethods().length > 0}
        <!-- Method Tabs -->
        <div class="method-tabs">
          {#each learnsetMethods() as method}
            <button
              class="method-tab"
              class:active={activeMethod === method}
              onclick={() => activeMethod = method}
            >
              {methodLabel(method)}
            </button>
          {/each}
        </div>

        <!-- Move Cards -->
        {#if activeLearnset().length > 0}
          <div class="move-list">
            {#each activeLearnset() as move (move.moveSlug + move.methodName + move.level)}
              <MoveCard
                name={move.moveName}
                slug={move.moveSlug}
                typeName={move.typeName}
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
      {:else}
        <p class="empty-text">No moves learned in this version.</p>
      {/if}
    </div>
  {/if}
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
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
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

  @media (max-width: 768px) {
    .content-grid { grid-template-columns: 1fr; }
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

  .info-box {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-4);
  }

  .info-box h3 {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-4) 0;
    border-bottom: 2px solid var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
  }

  /* ── Image ───────────────────────────────────────────────────────────────── */

  .image-box {
    position: relative;
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    display: flex;
    justify-content: center;
    align-items: center;
    padding: var(--spacing-6);
    flex: 1;
    min-height: 300px;
    transition: background-color 0.15s ease;
  }

  .image-box:has(.shiny-active) {
    background-color: color-mix(in srgb, var(--theme-bg-secondary) 85%, #b39ddb 15%);
  }

  .image-box img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  .shiny-toggle {
    position: absolute;
    bottom: var(--spacing-2);
    right: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 8px;
    padding: 4px 8px;
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-secondary);
    border: 2px solid var(--theme-border-primary);
    cursor: pointer;
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    transition: all 0.05s ease;
  }

  .shiny-toggle:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0px var(--theme-text-accent);
    transform: translate(-1px, -1px);
  }

  /* ── Version Selector ────────────────────────────────────────────────────── */

  .version-selector-container {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .version-selector-container label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
  }

  .version-select {
    flex: 1;
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 10px;
  }

  /* ── Flavor Text ─────────────────────────────────────────────────────────── */

  .flavor-text-box {
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.5;
    color: var(--theme-text-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .stats-box {
    flex: 1;
  }

  .data-box {
    flex: 1;
  }

  /* ── Data List ───────────────────────────────────────────────────────────── */

  .data-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .data-list li {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 14px;
  }

  .data-list .label { color: var(--theme-text-secondary); }
  .data-list .value { color: var(--theme-text-primary); font-weight: bold; }

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
    border-radius: 2px;
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
    margin-top: var(--spacing-4);
    padding-top: var(--spacing-4);
    border-top: 2px dashed var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
  }

  /* ── Type Matchup Section ───────────────────────────────────────────────── */

  .type-matchup-section {
    width: 100%;
  }

  /* ── Evolution Chain ─────────────────────────────────────────────────────── */

  .evo-tree {
    display: flex;
    flex-direction: column;
    align-items: center;
    overflow-x: auto;
    padding-bottom: var(--spacing-2);
  }

  /* Each node: card + optional children below */
  .evo-node-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* Linear single-child connector */
  .evo-conn-single {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-1);
    padding: var(--spacing-3) 0 var(--spacing-2);
    position: relative;
    z-index: 0;
  }

  .evo-conn-single::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 2px;
    background: var(--theme-border-primary);
    z-index: -1;
  }

  /* Vertical stem from parent down to horizontal bar */
  .evo-stem {
    width: 2px;
    height: var(--spacing-4);
    background: var(--theme-border-primary);
    flex-shrink: 0;
  }

  .evo-chunks-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    position: relative;
  }

  /* Row of branching child subtrees */
  .evo-branch-row {
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    gap: var(--spacing-4);
  }

  /* Each child subtree column */
  .evo-branch-child {
    display: flex;
    flex-direction: column;
    align-items: center;
    position: relative;
    overflow: visible;
  }

  /* Drop line from horizontal bar down to method tag */
  .evo-branch-child::after {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 2px;
    height: var(--spacing-4);
    background: var(--theme-border-primary);
  }

  /* Horizontal bar – first child: center rightward into gap */
  .evo-branch-child.first-branch::before {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    right: calc(-1 * var(--spacing-4) / 2);
    height: 2px;
    background: var(--theme-border-primary);
  }

  /* Horizontal bar – last child: left from gap to center */
  .evo-branch-child.last-branch::before {
    content: '';
    position: absolute;
    top: 0;
    right: 50%;
    left: calc(-1 * var(--spacing-4) / 2);
    height: 2px;
    background: var(--theme-border-primary);
  }

  /* Horizontal bar – middle children: full width + both gaps */
  .evo-branch-child.mid-branch::before {
    content: '';
    position: absolute;
    top: 0;
    left: calc(-1 * var(--spacing-4) / 2);
    right: calc(-1 * var(--spacing-4) / 2);
    height: 2px;
    background: var(--theme-border-primary);
  }

  /* Connector cell (method + arrow) in branching */
  .evo-conn-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-1);
    padding: var(--spacing-4) 0 var(--spacing-2);
    position: relative;
    z-index: 1;
  }

  /* Shared labels & arrow */
  .evo-method-tag {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    background: var(--theme-bg-secondary);
    padding: 3px 6px;
    text-align: center;
    white-space: nowrap;
    position: relative;
    z-index: 1;
  }

  .evo-item-link {
    color: var(--theme-text-accent);
    text-decoration: none;
    border-bottom: 1px solid transparent;
    transition: border-color 0.05s ease;
  }

  .evo-item-link:hover {
    border-bottom-color: var(--theme-text-accent);
  }

  .evo-down-arrow {
    font-size: 18px;
    color: var(--theme-text-secondary);
    line-height: 1;
    position: relative;
    z-index: 1;
  }

  /* Highlight current Pokémon's card */
  .evo-card-cell.is-current :global(.pokemon-card) {
    border-color: var(--theme-text-accent);
    box-shadow: 3px 3px 0px var(--theme-text-accent);
  }

  .chain-tab-bar {
    display: flex;
    gap: var(--spacing-2);
    margin-bottom: var(--spacing-4);
    flex-wrap: wrap;
  }

  .chain-tab-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: 4px 10px;
    border: 2px solid var(--theme-border-primary);
    background: transparent;
    color: var(--theme-text-secondary);
    cursor: pointer;
  }

  .chain-tab-btn.active {
    background-color: var(--theme-border-primary);
    color: var(--theme-bg-primary);
  }

  .alt-forms-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-4);
    justify-content: center;
  }

  /* ── Learnset ────────────────────────────────────────────────────────────── */

  .learnset-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: var(--spacing-4);
    border-bottom: 2px solid var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
  }

  .learnset-header h3 {
    margin: 0;
    border: none;
    padding: 0;
  }

  .version-label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
  }

  .method-tabs {
    display: flex;
    gap: var(--spacing-1);
    margin-bottom: var(--spacing-4);
    border-bottom: 2px solid var(--theme-border-primary);
    flex-wrap: wrap;
  }

  .method-tab {
    font-family: var(--font-display);
    font-size: 9px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: none;
    border-bottom: 3px solid transparent;
    color: var(--theme-text-secondary);
    cursor: pointer;
    margin-bottom: -2px;
    transition: all 0.05s ease;
  }

  .method-tab:hover {
    color: var(--theme-text-primary);
  }

  .method-tab.active {
    color: var(--theme-text-accent);
    border-bottom-color: var(--theme-text-accent);
  }

  .move-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .empty-text {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }
</style>
