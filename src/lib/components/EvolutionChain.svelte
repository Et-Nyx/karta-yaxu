<script lang="ts">
  import { enhance } from '$app/forms';
  import PokemonCard from './PokemonCard.svelte';

  // --- Types ---
  interface ChainForm {
    pokeapiId: number | null;
    name: string;
    slug: string;
    spriteUrl: string | null;
    isDefault: boolean;
    types: Array<{ name: string; hexColor: string | null }>;
    dexNumber: number;
  }

  interface ChainSpecies {
    speciesId: number;
    forms: ChainForm[];
  }

  interface EvolutionEdge {
    fromSpeciesId: number;
    fromName: string;
    fromSlug: string;
    fromSprite: string | null;
    toSpeciesId: number;
    toName: string;
    toSlug: string;
    toSprite: string | null;
    trigger: string;
    minLevel: number | null;
    minHappiness: number | null;
    itemName: string | null;
    itemSlug: string | null;
    timeOfDay: string | null;
    location: string | null;
    requiredSuffix: string | null;
  }

  interface EvoMethod {
    text: string | null;
    itemSlug: string | null;
    minLevel: number | null;
    minHappiness: number | null;
    timeOfDay: string | null;
  }

  // --- Component Props ---
  let {
    chainSpecies = [] as ChainSpecies[],
    evolutionChain = [] as EvolutionEdge[],
    activePokemon, // { slug: string; speciesId: number; id: number; }
    activeTab,     // Tab object: { kind: string; suffix?: string; forms?: ChainForm[] }
    variant = 'wiki' as 'wiki' | 'editor',
  } = $props();

  const REGIONAL_SUFFIXES_CHAIN = ['alola', 'galar', 'hisui', 'paldea'];

  // --- Helpers ---
  function chunkArray<T>(arr: T[], size: number): T[][] {
    const chunks: T[][] = [];
    for (let i = 0; i < arr.length; i += size) chunks.push(arr.slice(i, i + size));
    return chunks;
  }

  function primaryFormFor(sp: ChainSpecies, suffix: string | null): ChainForm {
    if (suffix) {
      const match = sp.forms.find(f => f.slug.includes('-' + suffix));
      if (match) return match;
    }
    return sp.forms.find(f => f.isDefault) ?? sp.forms[0];
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
        const y1 = rows[i].offsetTop + 2;
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

  // --- Derived Evo Tree ---
  let evoTree = $derived(() => {
    // 1. Get the current suffix for filtering
    const tabSuffix = activeTab?.kind === 'regional' ? activeTab.suffix : null;

    // 2. Filter evolution edges based on suffix
    const filteredEdges = evolutionChain.filter(edge => {
      if (tabSuffix) {
        // Under a regional tab (e.g. galar), show edges where the successor matches that suffix
        return edge.toSlug.endsWith('-' + tabSuffix) || edge.toSlug.includes('-' + tabSuffix + '-');
      } else {
        // Under the standard evolution tab, filter out edges that belong to other regional forms
        const hasRegionalSuffix = REGIONAL_SUFFIXES_CHAIN.some(suffix => {
          return edge.toSlug.endsWith('-' + suffix) || edge.toSlug.includes('-' + suffix + '-');
        });
        return !hasRegionalSuffix;
      }
    });

    // 3. Map species details by speciesId for displaying card info
    const speciesMap = new Map(chainSpecies.map(s => [s.speciesId, s]));

    // 4. Group children edges by predecessor slug
    type Child = {
      toSlug: string;
      toSpeciesId: number;
      method: EvoMethod;
    };
    const childrenOf = new Map<string, Child[]>();
    const childSlugs = new Set<string>();

    for (const edge of filteredEdges) {
      if (!childrenOf.has(edge.fromSlug)) {
        childrenOf.set(edge.fromSlug, []);
      }
      childrenOf.get(edge.fromSlug)!.push({
        toSlug: edge.toSlug,
        toSpeciesId: edge.toSpeciesId,
        method: {
          text: edge.trigger === 'unknown' ? null : (edge.itemName ?? edge.trigger.toUpperCase().replace(/-/g, ' ')),
          itemSlug: edge.itemSlug,
          minLevel: edge.minLevel,
          minHappiness: edge.minHappiness,
          timeOfDay: edge.timeOfDay,
        }
      });
      childSlugs.add(edge.toSlug);
    }

    // 5. Roots are predecessor slugs that do not appear as a child slug in the filtered list
    const parentSlugs = new Set(filteredEdges.map(e => e.fromSlug));
    const roots = [...parentSlugs].filter(slug => !childSlugs.has(slug));

    // Fallback: if there are no edges but a default form exists, make it a root
    if (roots.length === 0 && chainSpecies.length > 0) {
      const currentSp = chainSpecies.find(s => s.speciesId === activePokemon.speciesId);
      if (currentSp) {
        const pf = primaryFormFor(currentSp, tabSuffix);
        roots.push(pf.slug);
      }
    }

    return { speciesMap, childrenOf, roots };
  });

</script>

{#snippet evoNode(pokemonSlug: string)}
  {@const children = evoTree().childrenOf.get(pokemonSlug) ?? []}
  {@const pf = chainSpecies
    .flatMap(s => s.forms)
    .find(f => f.slug === pokemonSlug)}

  {#if pf}
    {@const isCurrent = pf.slug === activePokemon.slug}

    <div class="evo-node-wrap">
      <div class="evo-card-cell" class:is-current={isCurrent}>
        {#if variant === 'editor'}
          <PokemonCard
            pokemon={{
              pokeapiId: pf.pokeapiId ?? 0,
              name: pf.name,
              slug: pf.slug,
              spriteUrl: pf.spriteUrl,
              types: pf.types,
              dexNumber: pf.dexNumber,
            }}
            href="/editor/pokemon/{pf.slug}"
            highlighted={isCurrent}
          />
        {:else}
          <PokemonCard
            pokemon={{
              pokeapiId: pf.pokeapiId ?? 0,
              name: pf.name,
              slug: pf.slug,
              spriteUrl: pf.spriteUrl,
              types: pf.types,
              dexNumber: pf.dexNumber,
            }}
          />
        {/if}
      </div>

      {#if children.length === 1}
        {@const child = children[0]}

        <div class="evo-conn-single">
          {#if child.method.text}
            <span class="evo-method-tag">
              {#if variant === 'wiki' && child.method.itemSlug}
                <a href="/wiki/items/{child.method.itemSlug}" class="evo-item-link">{child.method.text}</a>
              {:else}
                {child.method.text}
              {/if}
            </span>
          {/if}
          <span class="evo-down-arrow">↓</span>
        </div>
        {@render evoNode(child.toSlug)}
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
                        {#if variant === 'wiki' && child.method.itemSlug}
                          <a href="/wiki/items/{child.method.itemSlug}" class="evo-item-link">{child.method.text}</a>
                        {:else}
                          {child.method.text}
                        {/if}
                      </span>
                    {/if}
                    <span class="evo-down-arrow">↓</span>
                  </div>
                  {@render evoNode(child.toSlug)}
                </div>
              {/each}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

{#if activeTab && 'forms' in activeTab}
  {@const currentSp = chainSpecies.find(s => s.speciesId === activePokemon.speciesId)}
  {@const baseForm = currentSp?.forms.find(f => f.isDefault) ?? currentSp?.forms[0]}
  <div class="evo-tree alternate-forms-tree">
    <div class="evo-node-wrap">
      {#if baseForm}
        <div class="evo-card-cell" class:is-current={baseForm.slug === activePokemon.slug}>
          {#if variant === 'editor'}
            <PokemonCard
              pokemon={{
                pokeapiId: baseForm.pokeapiId ?? 0,
                name: baseForm.name,
                slug: baseForm.slug,
                spriteUrl: baseForm.spriteUrl,
                types: baseForm.types,
                dexNumber: baseForm.dexNumber,
              }}
              href="/editor/pokemon/{baseForm.slug}"
              highlighted={baseForm.slug === activePokemon.slug}
            />
          {:else}
            <PokemonCard
              pokemon={{
                pokeapiId: baseForm.pokeapiId ?? 0,
                name: baseForm.name,
                slug: baseForm.slug,
                spriteUrl: baseForm.spriteUrl,
                types: baseForm.types,
                dexNumber: baseForm.dexNumber,
              }}
            />
          {/if}
        </div>
      {/if}

      {#if activeTab.forms.length === 1}
        {@const form = activeTab.forms[0]}
        <div class="evo-conn-cell">
          <span class="evo-method-tag">
            {form.slug.includes('-mega') ? 'MEGA EVOLUTION' : 'GIGANTAMAX'}
          </span>
          <span class="evo-down-arrow">↓</span>
        </div>

        <div class="evo-card-cell" class:is-current={form.slug === activePokemon.slug}>
          {#if variant === 'editor'}
            <PokemonCard
              pokemon={{
                pokeapiId: form.pokeapiId ?? 0,
                name: form.name,
                slug: form.slug,
                spriteUrl: form.spriteUrl,
                types: form.types,
                dexNumber: form.dexNumber,
              }}
              href="/editor/pokemon/{form.slug}"
              highlighted={form.slug === activePokemon.slug}
            />
          {:else}
            <PokemonCard
              pokemon={{
                pokeapiId: form.pokeapiId ?? 0,
                name: form.name,
                slug: form.slug,
                spriteUrl: form.spriteUrl,
                types: form.types,
                dexNumber: form.dexNumber,
              }}
            />
          {/if}
        </div>
      {:else if activeTab.forms.length > 1}
        <div class="evo-chunks-wrap" use:drawBranchLine>
          <div class="evo-stem"></div>
          <div class="evo-branch-row">
            {#each activeTab.forms as form, i}
              <div class="evo-branch-child"
                class:first-branch={i === 0}
                class:last-branch={i === activeTab.forms.length - 1}
                class:mid-branch={i > 0 && i < activeTab.forms.length - 1}
              >
                <div class="evo-conn-cell">
                  <span class="evo-method-tag">
                    {form.slug.includes('-mega') ? 'MEGA EVOLUTION' : 'GIGANTAMAX'}
                  </span>
                  <span class="evo-down-arrow">↓</span>
                </div>
                <div class="evo-card-cell" class:is-current={form.slug === activePokemon.slug}>
                  {#if variant === 'editor'}
                    <PokemonCard
                      pokemon={{
                        pokeapiId: form.pokeapiId ?? 0,
                        name: form.name,
                        slug: form.slug,
                        spriteUrl: form.spriteUrl,
                        types: form.types,
                        dexNumber: form.dexNumber,
                      }}
                      href="/editor/pokemon/{form.slug}"
                      highlighted={form.slug === activePokemon.slug}
                    />
                  {:else}
                    <PokemonCard
                      pokemon={{
                        pokeapiId: form.pokeapiId ?? 0,
                        name: form.name,
                        slug: form.slug,
                        spriteUrl: form.spriteUrl,
                        types: form.types,
                        dexNumber: form.dexNumber,
                      }}
                    />
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
{:else}
  <div class="evo-tree">
    {#each evoTree().roots as rootSlug}
      {@render evoNode(rootSlug)}
    {/each}
  </div>
{/if}

<style>
  /* Base structural nodes and connections */
  .evo-tree {
    display: flex; justify-content: center; align-items: flex-start; gap: 40px;
    padding: var(--spacing-4) 0; overflow-x: auto;
  }
  .evo-node-wrap {
    display: flex; flex-direction: column; align-items: center; position: relative;
  }
  .evo-card-cell {
    position: relative; z-index: 2; padding: 4px; border: 2px solid transparent;
  }
  .evo-card-cell.is-current {
    border-color: var(--theme-text-accent);
    background-color: rgba(var(--theme-accent-rgb, 74, 222, 128), 0.15);
  }
  .evo-conn-single {
    display: flex; flex-direction: column; align-items: center; gap: 4px; margin: 8px 0;
    position: relative; z-index: 2; min-height: 48px; justify-content: center;
  }
  .evo-conn-cell {
    display: flex; flex-direction: column; align-items: center; gap: 4px; margin: 8px 0;
    position: relative; z-index: 2; min-height: 48px; justify-content: center;
  }
  .evo-down-arrow {
    font-family: var(--font-display); font-size: 14px; color: var(--theme-text-muted);
    line-height: 1; margin-top: -2px;
  }
  .evo-method-tag {
    font-family: var(--font-display); font-size: 8px; letter-spacing: 0.05em;
    padding: 2px 6px; background-color: var(--theme-bg-tertiary);
    border: 1px solid var(--theme-border-primary); color: var(--theme-text-inverse);
    white-space: nowrap; max-width: 130px; overflow: hidden; text-overflow: ellipsis;
  }
  .evo-item-link {
    color: var(--theme-text-accent); text-decoration: none;
  }
  .evo-item-link:hover {
    text-decoration: underline;
  }

  /* Horizontal branching lines */
  .evo-chunks-wrap {
    position: relative; width: 100%; display: flex; flex-direction: column; align-items: center;
  }
  .evo-stem {
    width: 2px; height: 16px; background-color: var(--theme-border-primary);
    margin-bottom: -2px; position: relative; z-index: 1;
  }
  .evo-branch-row {
    display: flex; justify-content: center; gap: 24px; position: relative; z-index: 1; width: 100%;
  }
  .evo-branch-child {
    display: flex; flex-direction: column; align-items: center; position: relative;
    overflow: visible;
  }
  .evo-branch-child::after {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    width: 2px;
    height: 16px;
    background: var(--theme-border-primary);
  }
  .evo-branch-child.first-branch::before {
    content: '';
    position: absolute;
    top: 0;
    left: 50%;
    right: -12px;
    height: 2px;
    background: var(--theme-border-primary);
  }
  .evo-branch-child.last-branch::before {
    content: '';
    position: absolute;
    top: 0;
    right: 50%;
    left: -12px;
    height: 2px;
    background: var(--theme-border-primary);
  }
  .evo-branch-child.mid-branch::before {
    content: '';
    position: absolute;
    top: 0;
    left: -12px;
    right: -12px;
    height: 2px;
    background: var(--theme-border-primary);
  }
  .evo-branch-child.first-branch.last-branch::before {
    display: none;
  }

</style>
