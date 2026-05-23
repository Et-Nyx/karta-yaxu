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
    const links = evolutionChain;
    const speciesMap = new Map(chainSpecies.map(s => [s.speciesId, s]));
    type Child = { speciesId: number; method: EvoMethod; location: string | null; requiredSuffix: string | null };

    // Synthesize virtual edges for mid-chain species with regional form
    const syntheticLinks = [...links];
    for (const edge of links) {
      if (edge.location !== null) continue;
      const toSp = speciesMap.get(edge.toSpeciesId);
      if (!toSp) continue;
      for (const suffix of REGIONAL_SUFFIXES_CHAIN) {
        const hasRegionalForm = toSp.forms.some(f => f.slug.includes('-' + suffix));
        if (!hasRegionalForm) continue;
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

    const seen = new Set<string>();
    const locationDeduped = syntheticLinks.filter(l => {
      const key = `${l.fromSpeciesId}:${l.toSpeciesId}:${l.location ?? ''}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });

    const childrenOf = new Map<number, Child[]>();
    for (const l of locationDeduped) {
      if (!childrenOf.has(l.fromSpeciesId)) childrenOf.set(l.fromSpeciesId, []);
      childrenOf.get(l.fromSpeciesId)!.push({
        speciesId: l.toSpeciesId,
        method: {
          text: l.trigger === 'unknown' ? null : (l.itemName ?? l.trigger.toUpperCase().replace(/-/g, ' ')),
          itemSlug: l.itemSlug,
          minLevel: l.minLevel,
          minHappiness: l.minHappiness,
          timeOfDay: l.timeOfDay,
        },
        location: l.location,
        requiredSuffix: l.requiredSuffix,
      });
    }

    const childIds = new Set(locationDeduped.map(l => l.toSpeciesId));
    const roots = [...speciesMap.keys()].filter(id => !childIds.has(id));

    return { speciesMap, childrenOf, roots };
  });


</script>

{#snippet evoNode(speciesId: number, tabSuffix: string | null, edgeLocation: string | null)}
  {@const effectiveSuffix = edgeLocation ?? tabSuffix}
  {@const allChildren = evoTree().childrenOf.get(speciesId) ?? []}
  {@const children = allChildren.filter(child => {
    if (tabSuffix && child.location !== tabSuffix) {
      const hasRegionalVariant = allChildren.some(
        o => o.speciesId === child.speciesId && o.location === tabSuffix
      );
      if (hasRegionalVariant) return false;
    }
    return child.requiredSuffix === effectiveSuffix ||
      (child.requiredSuffix === null && !allChildren.some(
        o => o.speciesId !== child.speciesId && o.requiredSuffix === effectiveSuffix
      ));
  })}
  {@const sp = evoTree().speciesMap.get(speciesId)}
  {#if sp}
    {@const pf = primaryFormFor(sp, effectiveSuffix)}
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
        {@render evoNode(child.speciesId, tabSuffix, child.location)}
      {:else if children.length > 1}
        {@const chunks = chunkArray(children, 4)}
        <div class="evo-chunks-wrap" use:drawBranchLine>
          {#each chunks as chunk}
            <div class="evo-stem"></div>
            <div class="evo-branch-row">
              {#each chunk as child, i}
                {@const childSp = evoTree().speciesMap.get(child.speciesId)}
                {@const childPf = childSp ? primaryFormFor(childSp, effectiveSuffix) : null}
                {@const childIsCurrent = childPf && childPf.slug === activePokemon.slug}

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
    {#each evoTree().roots as rootId}
      {@const activeSuffix = activeTab?.kind === 'regional' ? activeTab.suffix : null}
      {@render evoNode(rootId, activeSuffix, null)}
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
