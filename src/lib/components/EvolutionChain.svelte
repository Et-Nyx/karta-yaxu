<script lang="ts">
  import { enhance } from '$app/forms';
  import PokemonCard from './PokemonCard.svelte';

  import RetroSelect from '$lib/components/RetroSelect.svelte';

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
    predecessorSlug = null as string | null,
    predecessorConditions = null as any,
    allPokemon = [] as any[],
    allItems = [] as any[],
    allMoves = [] as any[],
    allTypes = [] as any[],
  } = $props<{
    chainSpecies?: ChainSpecies[];
    evolutionChain?: EvolutionEdge[];
    activePokemon: { slug: string; speciesId: number; id: number; };
    activeTab?: any;
    variant?: 'wiki' | 'editor';
    predecessorSlug?: string | null;
    predecessorConditions?: any;
    allPokemon?: any[];
    allItems?: any[];
    allMoves?: any[];
    allTypes?: any[];
  }>();

  // Popover States
  let showPredecessorPopover = $state(false);
  let showConditionsPopover = $state(false);

  // Form states for Predecessor Form
  let selectedParentId = $state<string>('');
  let initialTrigger = $state<string>('level-up');

  // Form states for Conditions Form
  let activeTrigger = $state<string>('level-up');

  $effect(() => {
    if (predecessorSlug) {
      const parentObj = allPokemon.find((p: any) => p.slug === predecessorSlug);
      selectedParentId = parentObj ? String(parentObj.id) : '';
    } else {
      selectedParentId = '';
    }
  });

  $effect(() => {
    const t = predecessorConditions?.trigger ?? 'level-up';
    initialTrigger = t;
    activeTrigger = t;
  });

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
    const filteredEdges = evolutionChain.filter((edge: EvolutionEdge) => {
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
    const speciesMap = new Map(chainSpecies.map((s: ChainSpecies) => [s.speciesId, s]));

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
    const parentSlugs = new Set<string>(filteredEdges.map((e: EvolutionEdge) => e.fromSlug));
    const roots: string[] = [...parentSlugs].filter((slug: string) => !childSlugs.has(slug));

    // Fallback: if there are no edges but a default form exists, make it a root
    if (roots.length === 0 && chainSpecies.length > 0) {
      const currentSp = chainSpecies.find((s: ChainSpecies) => s.speciesId === activePokemon.speciesId);
      if (currentSp) {
        const pf = primaryFormFor(currentSp, tabSuffix);
        roots.push(pf.slug);
      }
    }

    return { speciesMap, childrenOf, roots };
  });

</script>

{#snippet conditionsPopover()}
  {#if showConditionsPopover}
    <div class="inline-popover conditions-popover">
      <div class="popover-header">
        <h3>EVOLUTION CONDITIONS</h3>
        <button type="button" class="btn-close" onclick={() => showConditionsPopover = false}>×</button>
      </div>
      <form action="?/updateConditions" method="post" use:enhance class="retro-form space-y-3">
        <!-- Hidden input to carry trigger -->
        <input type="hidden" name="trigger" value={activeTrigger} />

        <div class="field select-field">
          <label for="popoverTrigger">Trigger</label>
          <RetroSelect
            id="popoverTrigger"
            name="popoverTrigger"
            bind:value={activeTrigger}
            options={[
              { value: "level-up", label: "LEVEL UP" },
              { value: "use-item", label: "USE ITEM" },
              { value: "trade", label: "TRADE" },
              { value: "shed", label: "SHED" },
              { value: "other", label: "OTHER / CUSTOM" }
            ]}
            variant="editor"
          />
        </div>

        {#if activeTrigger === 'level-up'}
          <div class="grid grid-cols-2 gap-2">
            <div class="field">
              <label for="popoverMinLevel">MIN LEVEL</label>
              <input
                type="number"
                id="popoverMinLevel"
                name="minLevel"
                class="retro-input"
                placeholder="e.g. 16"
                value={predecessorConditions?.conditions?.minLevel ?? ""}
                min="1"
                max="100"
              />
            </div>
            <div class="field">
              <label for="popoverMinHappiness">MIN HAPPINESS</label>
              <input
                type="number"
                id="popoverMinHappiness"
                name="minHappiness"
                class="retro-input"
                placeholder="e.g. 220"
                value={predecessorConditions?.conditions?.minHappiness ?? ""}
                min="0"
                max="255"
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-2">
            <div class="field">
              <label for="popoverTimeOfDay">TIME OF DAY</label>
              <RetroSelect
                id="popoverTimeOfDay"
                name="timeOfDay"
                value={predecessorConditions?.conditions?.timeOfDay ?? ""}
                options={[
                  { value: "", label: "ANY" },
                  { value: "day", label: "DAY" },
                  { value: "night", label: "NIGHT" },
                  { value: "dusk", label: "DUSK" }
                ]}
                variant="editor"
              />
            </div>
            <div class="field">
              <label for="popoverGender">GENDER</label>
              <RetroSelect
                id="popoverGender"
                name="gender"
                value={predecessorConditions?.conditions?.gender ? String(predecessorConditions.conditions.gender) : ""}
                options={[
                  { value: "", label: "ANY" },
                  { value: "1", label: "FEMALE ONLY" },
                  { value: "2", label: "MALE ONLY" }
                ]}
                variant="editor"
              />
            </div>
          </div>

          <div class="field">
            <label for="popoverLocation">LOCATION</label>
            <input
              type="text"
              id="popoverLocation"
              name="location"
              class="retro-input"
              placeholder="e.g. Mount Coronet"
              value={predecessorConditions?.conditions?.location ?? ""}
            />
          </div>

        {:else if activeTrigger === 'use-item'}
          <div class="field">
            <label for="popoverItemId">EVOLUTION ITEM</label>
            <RetroSelect
              id="popoverItemId"
              name="itemId"
              value={predecessorConditions?.conditions?.itemId ? String(predecessorConditions.conditions.itemId) : ""}
              options={[
                { value: "", label: "--- SELECT ITEM ---" },
                ...allItems.map((i: any) => ({ value: String(i.id), label: i.name }))
              ]}
              variant="editor"
            />
          </div>

        {:else if activeTrigger === 'trade'}
          <div class="field">
            <label for="popoverHeldItemId">HELD ITEM ON TRADE</label>
            <RetroSelect
              id="popoverHeldItemId"
              name="heldItemId"
              value={predecessorConditions?.conditions?.heldItemId ? String(predecessorConditions.conditions.heldItemId) : ""}
              options={[
                { value: "", label: "--- NONE (FREE TRADE) ---" },
                ...allItems.map((i: any) => ({ value: String(i.id), label: i.name }))
              ]}
              variant="editor"
            />
          </div>

        {:else if activeTrigger === 'shed'}
          <div class="info-alert">
            Requires empty slot and standard Pokéball.
          </div>

        {:else}
          <div class="field">
            <label for="popoverLocationNote">CUSTOM NOTE</label>
            <input
              type="text"
              id="popoverLocationNote"
              name="location"
              class="retro-input"
              placeholder="e.g. Spin in circles"
              value={predecessorConditions?.conditions?.location ?? ""}
            />
          </div>
        {/if}

        <details class="retro-details mt-1">
          <summary class="text-[9px] cursor-pointer select-none">ADVANCED CONDITIONS</summary>
          <div class="space-y-2 pt-2">
            <div class="field">
              <label for="popoverKnownMoveId">KNOWN MOVE</label>
              <RetroSelect
                id="popoverKnownMoveId"
                name="knownMoveId"
                value={predecessorConditions?.conditions?.knownMoveId ? String(predecessorConditions.conditions.knownMoveId) : ""}
                options={[
                  { value: "", label: "--- NONE ---" },
                  ...allMoves.map((m: any) => ({ value: String(m.id), label: m.name }))
                ]}
                variant="editor"
              />
            </div>
            <div class="field">
              <label for="popoverKnownMoveTypeId">MOVE TYPE</label>
              <RetroSelect
                id="popoverKnownMoveTypeId"
                name="knownMoveTypeId"
                value={predecessorConditions?.conditions?.knownMoveTypeId ? String(predecessorConditions.conditions.knownMoveTypeId) : ""}
                options={[
                  { value: "", label: "--- NONE ---" },
                  ...allTypes.map((t: any) => ({ value: String(t.id), label: t.name }))
                ]}
                variant="editor"
              />
            </div>
          </div>
        </details>

        <button type="submit" class="btn btn-primary w-full mt-2">
          SAVE CONDITIONS
        </button>
      </form>
    </div>
  {/if}
{/snippet}

{#snippet evoNode(pokemonSlug: string)}
  {@const children = evoTree().childrenOf.get(pokemonSlug) ?? []}
  {@const pf = chainSpecies
    .flatMap((s: ChainSpecies) => s.forms)
    .find((f: ChainForm) => f.slug === pokemonSlug)}

  {#if pf}
    {@const isCurrent = pf.slug === activePokemon.slug}
    {@const isPredecessor = pf.slug === predecessorSlug}

    <div class="evo-node-wrap">
      <div class="evo-card-cell" class:is-current={isCurrent} class:is-predecessor={variant === 'editor' && isPredecessor}>
        {#if variant === 'editor'}
          <div class="card-relative-container">
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
            {#if isPredecessor}
              <button 
                type="button" 
                class="btn-edit-relation-overlay"
                onclick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  showPredecessorPopover = !showPredecessorPopover;
                }}
              >
                EDIT LINK
              </button>

              {#if showPredecessorPopover}
                <div class="inline-popover predecessor-popover">
                  <div class="popover-header">
                    <h3>EDIT RELATION</h3>
                    <button type="button" class="btn-close" onclick={() => showPredecessorPopover = false}>×</button>
                  </div>
                  <form action="?/setPredecessor" method="post" use:enhance class="retro-form space-y-4">
                    <div class="field">
                      <label for="parentPokemonId">Select Predecessor</label>
                      <div class="flex gap-2 items-center">
                        <div class="flex-grow">
                          <RetroSelect
                            id="parentPokemonId"
                            name="parentPokemonId"
                            bind:value={selectedParentId}
                            options={[
                              { value: "", label: "--- NONE (BASIC POKÉMON) ---" },
                              ...allPokemon
                                .filter((p: any) => p.id !== activePokemon.id)
                                .map((p: any) => ({ value: String(p.id), label: p.name }))
                            ]}
                            variant="editor"
                          />
                        </div>
                        <button type="submit" formAction="?/clearPredecessor" class="btn-clear-relation flex-shrink-0">
                          CLEAR
                        </button>
                      </div>
                    </div>

                    <div class="field">
                      <label for="trigger">Initial Trigger</label>
                      <RetroSelect
                        id="trigger"
                        name="trigger"
                        bind:value={initialTrigger}
                        options={[
                          { value: "level-up", label: "LEVEL UP" },
                          { value: "use-item", label: "USE ITEM" },
                          { value: "trade", label: "TRADE" },
                          { value: "shed", label: "SHED" },
                          { value: "other", label: "OTHER / CUSTOM" }
                        ]}
                        variant="editor"
                      />
                    </div>

                    <button type="submit" class="btn btn-primary w-full mt-2">
                      UPDATE PREDECESSOR
                    </button>
                  </form>
                </div>
              {/if}
            {/if}
          </div>
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
            {#if variant === 'editor' && child.toSlug === activePokemon.slug}
              <div class="card-relative-container">
                <button 
                  type="button" 
                  class="evo-method-tag interactive-tag" 
                  class:is-active-relation={true}
                  onclick={() => showConditionsPopover = !showConditionsPopover}
                >
                  {child.method.text} ✏️
                </button>
                {@render conditionsPopover()}
              </div>
            {:else}
              <span class="evo-method-tag">
                {#if variant === 'wiki' && child.method.itemSlug}
                  <a href="/wiki/items/{child.method.itemSlug}" class="evo-item-link">{child.method.text}</a>
                {:else}
                  {child.method.text}
                {/if}
              </span>
            {/if}
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
                      {#if variant === 'editor' && child.toSlug === activePokemon.slug}
                        <div class="card-relative-container">
                          <button 
                            type="button" 
                            class="evo-method-tag interactive-tag" 
                            class:is-active-relation={true}
                            onclick={() => showConditionsPopover = !showConditionsPopover}
                          >
                            {child.method.text} ✏️
                          </button>
                          {@render conditionsPopover()}
                        </div>
                      {:else}
                        <span class="evo-method-tag">
                          {#if variant === 'wiki' && child.method.itemSlug}
                            <a href="/wiki/items/{child.method.itemSlug}" class="evo-item-link">{child.method.text}</a>
                          {:else}
                            {child.method.text}
                          {/if}
                        </span>
                      {/if}
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
  {@const currentSp = chainSpecies.find((s: ChainSpecies) => s.speciesId === activePokemon.speciesId)}
  {@const baseForm = currentSp?.forms.find((f: ChainForm) => f.isDefault) ?? currentSp?.forms[0]}
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
  <div class="evo-tree" class:has-dashed-predecessor={variant === 'editor' && !predecessorSlug}>
    {#if variant === 'editor' && !predecessorSlug}
      <div class="evo-node-wrap predecessor-creator-wrap">
        <div class="card-relative-container">
          <button 
            type="button" 
            class="dashed-predecessor-slot" 
            onclick={() => showPredecessorPopover = !showPredecessorPopover}
          >
            <span class="plus-icon">+</span>
            <span class="label">ADD PREDECESSOR</span>
          </button>

          {#if showPredecessorPopover}
            <div class="inline-popover predecessor-popover">
              <div class="popover-header">
                <h3>LINK PREDECESSOR</h3>
                <button type="button" class="btn-close" onclick={() => showPredecessorPopover = false}>×</button>
              </div>
              <form action="?/setPredecessor" method="post" use:enhance class="retro-form space-y-4">
                <div class="field">
                  <label for="parentPokemonId">Select Predecessor</label>
                  <RetroSelect
                    id="parentPokemonId"
                    name="parentPokemonId"
                    bind:value={selectedParentId}
                    options={[
                      { value: "", label: "--- SELECT POKÉMON ---" },
                      ...allPokemon
                        .filter((p: any) => p.id !== activePokemon.id)
                        .map((p: any) => ({ value: String(p.id), label: p.name }))
                    ]}
                    variant="editor"
                  />
                </div>

                <div class="field">
                  <label for="trigger">Initial Trigger</label>
                  <RetroSelect
                    id="trigger"
                    name="trigger"
                    bind:value={initialTrigger}
                    options={[
                      { value: "level-up", label: "LEVEL UP" },
                      { value: "use-item", label: "USE ITEM" },
                      { value: "trade", label: "TRADE" },
                      { value: "shed", label: "SHED" },
                      { value: "other", label: "OTHER / CUSTOM" }
                    ]}
                    variant="editor"
                  />
                </div>

                <button type="submit" class="btn btn-primary w-full mt-2">
                  LINK PREDECESSOR
                </button>
              </form>
            </div>
          {/if}
        </div>

        <div class="evo-conn-single">
          <span class="evo-method-tag">BASIC POKÉMON</span>
          <span class="evo-down-arrow">↓</span>
        </div>
      </div>
    {/if}

    <div class="evo-tree-roots-row">
      {#each evoTree().roots as rootSlug}
        {@render evoNode(rootSlug)}
      {/each}
    </div>
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

  /* Interactive styling for Editor Mode */
  .card-relative-container {
    position: relative;
    display: inline-block;
  }

  .evo-card-cell.is-predecessor {
    border-color: var(--theme-border-primary);
    box-shadow: 0 0 10px rgba(139, 172, 15, 0.4);
  }

  .btn-edit-relation-overlay {
    position: absolute;
    bottom: 6px;
    left: 50%;
    transform: translateX(-50%);
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-display);
    font-size: 8px;
    padding: 3px 6px;
    cursor: pointer;
    box-shadow: 1.5px 1.5px 0 var(--theme-border-primary);
    z-index: 10;
    white-space: nowrap;
    opacity: 0.9;
    transition: all 0.1s ease;
  }

  .btn-edit-relation-overlay:hover {
    background-color: var(--theme-bg-hover);
    transform: translateX(-50%) translate(-0.5px, -0.5px);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
  }

  .btn-edit-relation-overlay:active {
    transform: translateX(-50%) translate(0.5px, 0.5px);
    box-shadow: 0.5px 0.5px 0 var(--theme-border-primary);
  }

  button.evo-method-tag.interactive-tag {
    appearance: none;
    cursor: pointer;
    background-color: var(--theme-bg-secondary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    box-shadow: 1.5px 1.5px 0 var(--theme-border-primary);
    transition: all 0.1s ease;
  }

  button.evo-method-tag.interactive-tag:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
    transform: translate(-1px, -1px);
    box-shadow: 2.5px 2.5px 0 var(--theme-border-primary);
  }

  button.evo-method-tag.interactive-tag:active {
    transform: translate(1px, 1px);
    box-shadow: 0.5px 0.5px 0 var(--theme-border-primary);
  }

  /* Dashed Predecessor creator card styles */
  .dashed-predecessor-slot {
    appearance: none;
    width: 120px;
    height: 140px;
    border: 2px dashed var(--theme-border-primary);
    background-color: transparent;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 8px;
    color: var(--theme-text-secondary);
    font-family: var(--font-display);
    font-size: 8px;
    transition: all 0.15s ease;
  }

  .dashed-predecessor-slot:hover {
    background-color: rgba(139, 172, 15, 0.05);
    color: var(--theme-text-primary);
    border-style: solid;
  }

  .dashed-predecessor-slot .plus-icon {
    font-size: 24px;
    margin-bottom: 8px;
    line-height: 1;
  }

  .predecessor-creator-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 12px;
  }

  /* Inline popover elements */
  .inline-popover {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 100;
    width: 250px;
    background-color: var(--theme-bg-secondary);
    border: 4px double var(--theme-border-primary);
    padding: var(--spacing-3);
    box-shadow: 4px 4px 0 var(--theme-border-primary);
    text-align: left;
    overflow: visible;
  }

  .conditions-popover {
    bottom: auto;
    top: calc(100% + 8px);
  }

  .predecessor-popover {
    bottom: calc(100% + 8px);
  }

  .popover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px dashed var(--theme-border-primary);
    padding-bottom: 4px;
    margin-bottom: 8px;
  }

  .popover-header h3 {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .btn-close {
    appearance: none;
    border: none;
    background: none;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: bold;
    color: var(--theme-text-muted);
    cursor: pointer;
    line-height: 1;
    padding: 0 4px;
  }

  .btn-close:hover {
    color: var(--theme-text-primary);
  }

  .retro-form .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .retro-form .field label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
  }

  .retro-form input.retro-input {
    background-color: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-body);
    font-size: 11px;
    padding: 4px 6px;
    outline: none;
    border-radius: 0;
  }

  .retro-form input.retro-input:focus {
    border-color: var(--theme-text-accent);
  }

  .btn-clear-relation {
    appearance: none;
    background-color: var(--theme-bg-tertiary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-display);
    font-size: 8px;
    padding: 4px 8px;
    cursor: pointer;
  }

  .btn-clear-relation:hover {
    background-color: var(--theme-bg-hover);
  }

  .info-alert {
    background-color: var(--theme-bg-tertiary);
    border-left: 3px solid var(--theme-border-primary);
    padding: 6px;
    font-size: 9px;
    line-height: 1.3;
    color: var(--theme-text-secondary);
  }

  .retro-details {
    border: 1px solid var(--theme-border-primary);
    background-color: var(--theme-bg-tertiary);
    padding: 4px;
  }

  .retro-details summary {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    outline: none;
  }

  .retro-details summary:hover {
    color: var(--theme-text-accent);
  }
</style>
