<script lang="ts">
  import { enhance } from '$app/forms';
  import PokemonCard from './PokemonCard.svelte';

  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import RetroButton from './RetroButton.svelte';

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
    predecessorId = $bindable(''),
    evoTrigger = $bindable('level-up'),
    minLevel = $bindable(''),
    minHappiness = $bindable(''),
    timeOfDay = $bindable(''),
    gender = $bindable(''),
    location = $bindable(''),
    itemId = $bindable(''),
    heldItemId = $bindable(''),
    knownMoveId = $bindable(''),
    knownMoveTypeId = $bindable(''),
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
    predecessorId?: string;
    evoTrigger?: string;
    minLevel?: string;
    minHappiness?: string;
    timeOfDay?: string;
    gender?: string;
    location?: string;
    itemId?: string;
    heldItemId?: string;
    knownMoveId?: string;
    knownMoveTypeId?: string;
    allPokemon?: any[];
    allItems?: any[];
    allMoves?: any[];
    allTypes?: any[];
  }>();

  // Popover States
  let showPredecessorPopover = $state(false);
  let showConditionsPopover = $state(false);

  let predecessorSlug = $derived.by(() => {
    if (!predecessorId) return null;
    const parentObj = allPokemon.find((p: any) => p.id === Number(predecessorId));
    return parentObj ? parentObj.slug : null;
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

{#snippet predecessorPopover()}
  <div class="sidebar-editor-card">
    <div class="popover-header">
      <h3>{#if predecessorSlug}EDIT RELATION{:else}LINK PREDECESSOR{/if}</h3>
      <button type="button" class="btn-close" onclick={() => showPredecessorPopover = false}>×</button>
    </div>
    <div class="retro-form space-y-4">
      <div class="field">
        <label for="parentPokemonId">Select Predecessor</label>
        <div class="flex gap-2 items-center">
          <div class="flex-grow">
            <RetroSelect
              id="parentPokemonId"
              name="parentPokemonId"
              bind:value={predecessorId}
              options={[
                { value: "", label: predecessorSlug ? "--- NONE (BASIC POKÉMON) ---" : "--- SELECT POKÉMON ---" },
                ...allPokemon
                  .filter((p: any) => p.id !== activePokemon.id)
                  .map((p: any) => ({ value: String(p.id), label: p.name }))
              ]}
              variant="editor"
            />
          </div>
          {#if predecessorSlug}
            <RetroButton type="button" onclick={() => predecessorId = ""} variant="secondary" class="flex-shrink-0">
              CLEAR
            </RetroButton>
          {/if}
        </div>
      </div>

      <div class="field">
        <label for="trigger">Initial Trigger</label>
        <RetroSelect
          id="trigger"
          name="trigger"
          bind:value={evoTrigger}
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
    </div>
  </div>
{/snippet}

{#snippet conditionsPopover()}
  <div class="sidebar-editor-card">
    <div class="popover-header">
      <h3>EVOLUTION CONDITIONS</h3>
      <button type="button" class="btn-close" onclick={() => showConditionsPopover = false}>×</button>
    </div>
    <div class="retro-form space-y-3">
      <div class="field select-field">
        <label for="popoverTrigger">Trigger</label>
        <RetroSelect
          id="popoverTrigger"
          name="popoverTrigger"
          bind:value={evoTrigger}
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

      {#if evoTrigger === 'level-up'}
        <div class="grid grid-cols-2 gap-2">
          <div class="field">
            <label for="popoverMinLevel">MIN LEVEL</label>
            <input
              type="number"
              id="popoverMinLevel"
              name="minLevel"
              class="retro-input"
              placeholder="e.g. 16"
              bind:value={minLevel}
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
              bind:value={minHappiness}
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
              bind:value={timeOfDay}
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
              bind:value={gender}
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
            bind:value={location}
          />
        </div>

      {:else if evoTrigger === 'use-item'}
        <div class="field">
          <label for="popoverItemId">EVOLUTION ITEM</label>
          <RetroSelect
            id="popoverItemId"
            name="itemId"
            bind:value={itemId}
            options={[
              { value: "", label: "--- SELECT ITEM ---" },
              ...allItems.map((i: any) => ({ value: String(i.id), label: i.name }))
            ]}
            variant="editor"
          />
        </div>

      {:else if evoTrigger === 'trade'}
        <div class="field">
          <label for="popoverHeldItemId">HELD ITEM ON TRADE</label>
          <RetroSelect
            id="popoverHeldItemId"
            name="heldItemId"
            bind:value={heldItemId}
            options={[
              { value: "", label: "--- NONE (FREE TRADE) ---" },
              ...allItems.map((i: any) => ({ value: String(i.id), label: i.name }))
            ]}
            variant="editor"
          />
        </div>

      {:else if evoTrigger === 'shed'}
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
            bind:value={location}
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
              bind:value={knownMoveId}
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
              bind:value={knownMoveTypeId}
              options={[
                { value: "", label: "--- NONE ---" },
                ...allTypes.map((t: any) => ({ value: String(t.id), label: t.name }))
              ]}
              variant="editor"
            />
          </div>
        </div>
      </details>
    </div>
  </div>
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
            {#if isCurrent}
              <PokemonCard
                pokemon={{
                  pokeapiId: pf.pokeapiId ?? 0,
                  name: pf.name,
                  slug: pf.slug,
                  spriteUrl: pf.spriteUrl,
                  types: pf.types,
                  dexNumber: pf.dexNumber,
                }}
                highlighted={true}
              />
            {:else if isPredecessor}
              <PokemonCard
                pokemon={{
                  pokeapiId: pf.pokeapiId ?? 0,
                  name: pf.name,
                  slug: pf.slug,
                  spriteUrl: pf.spriteUrl,
                  types: pf.types,
                  dexNumber: pf.dexNumber,
                }}
                onclick={() => {
                  showPredecessorPopover = !showPredecessorPopover;
                  showConditionsPopover = false;
                }}
                highlighted={showPredecessorPopover}
                dashed={true}
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
                href="/editor/pokemon/{pf.slug}"
              />
            {/if}
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
<div class="evo-chain-layout" class:has-sidebar={variant === 'editor' && showPredecessorPopover}>
  {#if variant === 'editor' && showPredecessorPopover}
    <div class="evo-chain-sidebar-left">
      {@render predecessorPopover()}
    </div>
  {/if}

  <div class="evo-chain-tree-column">
    {#if activeTab && 'forms' in activeTab}
  {@const currentSp = chainSpecies.find((s: ChainSpecies) => s.speciesId === activePokemon.speciesId)}
  {@const baseForm = currentSp?.forms.find((f: ChainForm) => f.isDefault) ?? currentSp?.forms[0]}
  <div class="evo-tree alternate-forms-tree">
    <div class="evo-node-wrap">
      {#if baseForm}
        <div class="evo-card-cell" class:is-current={baseForm.slug === activePokemon.slug}>
          {#if variant === 'editor'}
            {#if baseForm.slug === activePokemon.slug}
              <PokemonCard
                pokemon={{
                  pokeapiId: baseForm.pokeapiId ?? 0,
                  name: baseForm.name,
                  slug: baseForm.slug,
                  spriteUrl: baseForm.spriteUrl,
                  types: baseForm.types,
                  dexNumber: baseForm.dexNumber,
                }}
                highlighted={true}
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
                href="/editor/pokemon/{baseForm.slug}"
              />
            {/if}
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
            {#if form.slug === activePokemon.slug}
              <PokemonCard
                pokemon={{
                  pokeapiId: form.pokeapiId ?? 0,
                  name: form.name,
                  slug: form.slug,
                  spriteUrl: form.spriteUrl,
                  types: form.types,
                  dexNumber: form.dexNumber,
                }}
                highlighted={true}
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
                href="/editor/pokemon/{form.slug}"
              />
            {/if}
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
                    {#if form.slug === activePokemon.slug}
                      <PokemonCard
                        pokemon={{
                          pokeapiId: form.pokeapiId ?? 0,
                          name: form.name,
                          slug: form.slug,
                          spriteUrl: form.spriteUrl,
                          types: form.types,
                          dexNumber: form.dexNumber,
                        }}
                        highlighted={true}
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
                        href="/editor/pokemon/{form.slug}"
                      />
                    {/if}
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
            class:is-active-relation={showPredecessorPopover}
            onclick={() => {
              showPredecessorPopover = !showPredecessorPopover;
              showConditionsPopover = false;
            }}
          >
            <span class="plus-icon">+</span>
            <span class="label">ADD PREDECESSOR</span>
          </button>
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
  </div>
  {#if variant === 'editor' && showPredecessorPopover}
    <div class="evo-chain-sidebar-right">
      {@render conditionsPopover()}
    </div>
  {/if}
</div>

<style>
  /* Base structural nodes and connections */
  .evo-tree {
    display: flex;
    align-items: flex-start;
    gap: 40px;
    padding: var(--spacing-4) 0;
    width: max-content;
    margin: 0 auto;
  }
  .evo-tree.has-dashed-predecessor {
    flex-direction: column;
    align-items: center;
    gap: 0;
    margin: 0 auto;
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






  /* Dashed Predecessor creator card styles */
  .dashed-predecessor-slot {
    appearance: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-3);
    min-width: 140px;
    min-height: 200px;
    background-color: var(--theme-bg-secondary);
    border: 3px dashed var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    cursor: pointer;
    font-family: var(--font-display);
    position: relative;
    transition: none;
  }

  .dashed-predecessor-slot.is-active-relation {
    border-color: var(--theme-text-accent);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }

  .dashed-predecessor-slot::after {
    content: "LINK PREDECESSOR";
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: rgba(15, 56, 15, 0.9); /* Retro deep forest green overlay */
    color: var(--theme-text-accent);
    font-family: var(--font-display);
    font-size: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    line-height: 1.5;
    padding: var(--spacing-3);
    opacity: 0;
    transition: opacity 0.1s steps(2); /* Crisp retro step-fade transition */
    z-index: 20;
    pointer-events: none;
    border: 3px dashed var(--theme-text-accent);
    margin: -3px; /* Align perfectly with outer border */
    text-shadow: 1px 1px 0px #0f380f;
  }

  .dashed-predecessor-slot:hover::after {
    opacity: 1;
  }

  .dashed-predecessor-slot .plus-icon {
    font-size: 24px;
    font-weight: bold;
    color: var(--theme-text-secondary);
    margin-bottom: var(--spacing-2);
    line-height: 1;
  }

  .dashed-predecessor-slot .label {
    font-size: 8px;
    color: var(--theme-text-secondary);
    text-align: center;
  }

  .predecessor-creator-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 12px;
  }

  /* Sidebar Layout container */
  .evo-chain-layout {
    position: relative;
    width: 100%;
  }

  .evo-chain-tree-column {
    width: 100%;
  }

  .evo-chain-sidebar-left {
    position: absolute;
    left: 0;
    top: var(--spacing-4);
    width: 300px;
    z-index: 20;
  }

  .evo-chain-sidebar-right {
    position: absolute;
    right: 0;
    top: var(--spacing-4);
    width: 300px;
    z-index: 20;
  }

  .sidebar-editor-card {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-4);
    text-align: left;
  }

  @media (max-width: 1100px) {
    .evo-chain-layout {
      display: flex;
      flex-direction: column;
      align-items: stretch;
      gap: var(--spacing-6);
    }
    .evo-chain-sidebar-left,
    .evo-chain-sidebar-right {
      position: static;
      width: 100%;
    }
  }

  .popover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--theme-bg-tertiary);
    border-bottom: 2px solid var(--theme-border-primary);
    margin-top: calc(-1 * var(--spacing-4));
    margin-left: calc(-1 * var(--spacing-4));
    margin-right: calc(-1 * var(--spacing-4));
    margin-bottom: var(--spacing-4);
    padding: var(--spacing-3) var(--spacing-4);
  }

  .popover-header h3 {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-inverse);
    margin: 0;
  }

  .btn-close {
    appearance: none;
    border: none;
    background: none;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: bold;
    color: var(--theme-text-inverse);
    cursor: pointer;
    line-height: 1;
    padding: 0 4px;
    opacity: 0.8;
  }

  .btn-close:hover {
    opacity: 1;
    color: var(--theme-text-accent);
  }

  .retro-form .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
    margin-bottom: var(--spacing-3);
  }

  .retro-form .field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .retro-form input.retro-input {
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-mono);
    font-size: 11px;
    padding: var(--spacing-1) var(--spacing-2);
    outline: none;
    border-radius: 0;
    box-sizing: border-box;
    width: 100%;
  }

  .retro-form input.retro-input:focus {
    border-color: var(--theme-text-accent);
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
    border: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-tertiary);
    padding: var(--spacing-2) var(--spacing-3);
  }

  .retro-details summary {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    outline: none;
    cursor: pointer;
  }

  .retro-details summary:hover {
    color: var(--theme-text-accent);
  }
</style>
