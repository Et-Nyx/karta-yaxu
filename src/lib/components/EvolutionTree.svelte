<script lang="ts">
  import PokemonCard from './PokemonCard.svelte';

  export type EvoNode = {
    speciesId: number;
    pokeapiId: number;
    name: string;
    slug: string;
    spriteDefault: string | null;
    isBaby: boolean;
    order: number | null;
    types: string[];
  };

  export type EvoEdge = {
    id: number;
    fromSpeciesId: number;
    toSpeciesId: number;
    trigger: string;
    minLevel: number | null;
    itemName?: string;
  };

  type EvoTreeNode = EvoNode & {
    edges?: EvoEdge[];
    children: EvoTreeNode[];
  };

  let { nodes = [], edges = [] } = $props<{ nodes: EvoNode[], edges: EvoEdge[] }>();

  function buildTree(currentSpeciesId: number): EvoTreeNode | null {
    const node = nodes.find(n => n.speciesId === currentSpeciesId);
    if (!node || node.pokeapiId == null || node.name == null) return null;

    const childrenEdges = edges.filter(e => e.fromSpeciesId === currentSpeciesId);

    const groupedEdges = new Map<number, EvoEdge>();
    for (const e of childrenEdges) {
      if (!groupedEdges.has(e.toSpeciesId)) {
        groupedEdges.set(e.toSpeciesId, e);
      }
    }

    const children: EvoTreeNode[] = Array.from(groupedEdges.entries())
      .map(([toSpeciesId, edge]) => {
        const childTree = buildTree(toSpeciesId);
        if (!childTree) return null;
        return { ...childTree, edges: [edge] };
      })
      .filter((c): c is EvoTreeNode => c !== null);

    return { ...node, children };
  }

  let roots = $derived(() => {
    const toIds = new Set(edges.map(e => e.toSpeciesId));
    const rootNodes = nodes.filter(n => !toIds.has(n.speciesId));
    return rootNodes.map(r => buildTree(r.speciesId)).filter((r): r is EvoTreeNode => r !== null);
  });

  function getLayoutMode(childCount: number): string {
    if (childCount === 0) return 'single';
    if (childCount === 1) return 'linear';
    if (childCount <= 4) return 'branched';
    return 'multi-row';
  }

  function chunkArray(arr: any[], size: number): any[][] {
    const chunks = [];
    for (let i = 0; i < arr.length; i += size) {
      chunks.push(arr.slice(i, i + size));
    }
    return chunks;
  }

  function formatTrigger(trigger: string, minLevel: number | null): string {
    const label = trigger.toUpperCase().replace(/-/g, ' ');
    if (minLevel) return `${label} Lvl ${minLevel}`;
    return label;
  }
</script>

<div class="evolution-tree">
  {#each roots() as rootTree}
    {#if rootTree}
      <div class="tree-container">
        {@render RenderNode(rootTree)}
      </div>
    {/if}
  {/each}
</div>

{#snippet RenderNode(tree)}
  <div class="node-wrapper">
    <div class="node-card-wrapper">
      <PokemonCard
        pokemon={{
          pokeapiId: tree.pokeapiId,
          name: tree.name,
          slug: tree.slug,
          spriteUrl: tree.spriteDefault,
          types: tree.types || []
        }}
      />
    </div>
    {@render RenderConnections(tree)}
  </div>
{/snippet}

{#snippet RenderConnections(tree)}
  {@const childCount = tree.children.length}
  {@const layout = getLayoutMode(childCount)}
  {@const rows = layout === 'multi-row' ? chunkArray(tree.children, 4) : []}

  {#if layout === 'single'}
    <!-- No children -->

  {:else if layout === 'linear'}
    <!-- Single child: vertical chain -->
    <div class="linear-connector">
      <span class="arrow-down">↓</span>
      <span class="trigger-label">{formatTrigger(tree.children[0].edges![0].trigger, tree.children[0].edges![0].minLevel)}</span>
    </div>
    {@render RenderNode(tree.children[0])}

  {:else if layout === 'branched'}
    <!-- 2-4 children: single horizontal row -->
    <div class="branched-connector">
      <div class="connector-vertical-short"></div>
      <div class="horizontal-bar">
        {#each tree.children as child}
          <div class="branch-position">
            <span class="arrow-down">↓</span>
            <div class="branch-trigger">
              <span class="trigger-label">{formatTrigger(child.edges![0].trigger, child.edges![0].minLevel)}</span>
            </div>
            <div class="branch-card">
              <PokemonCard
                pokemon={{
                  pokeapiId: child.pokeapiId,
                  name: child.name,
                  slug: child.slug,
                  spriteUrl: child.spriteDefault,
                  types: child.types || []
                }}
              />
              {@render RenderConnections(child)}
            </div>
          </div>
        {/each}
      </div>
    </div>

  {:else}
    <!-- 5+ children: multi-row layout -->
    <div class="multi-row-connector">
      <div class="connector-vertical-short"></div>
      {#each rows as row, rowIndex}
        <div class="row-section">
          <div class="horizontal-bar">
            {#each row as child}
              <div class="branch-position">
                <span class="arrow-down">↓</span>
                <div class="branch-trigger">
                  <span class="trigger-label">{formatTrigger(child.edges![0].trigger, child.edges![0].minLevel)}</span>
                </div>
                <div class="branch-card">
                  <PokemonCard
                    pokemon={{
                      pokeapiId: child.pokeapiId,
                      name: child.name,
                      slug: child.slug,
                      spriteUrl: child.spriteDefault,
                      types: child.types || []
                    }}
                  />
                  {@render RenderConnections(child)}
                </div>
              </div>
            {/each}
          </div>
          {#if rowIndex < rows.length - 1}
            <div class="connector-vertical-between-rows"></div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

<style>
  /* ── Base ─────────────────────────────────────────────────── */
  .evolution-tree {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-6);
    overflow-x: hidden;
  }

  .tree-container {
    display: flex;
    justify-content: center;
    min-width: fit-content;
  }

  .node-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .node-card-wrapper {
    z-index: 2;
  }

  /* ── Linear Layout ────────────────────────────────────────── */
  .linear-connector {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--spacing-2) 0;
  }

  .arrow-down {
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-secondary);
    line-height: 1;
  }

  /* ── Branched Layout ──────────────────────────────────────── */
  .branched-connector {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  /* ── Branched & Multi-Row Shared ──────────────────────────── */
  .connector-vertical-short {
    width: 2px;
    height: var(--spacing-4);
    background-color: var(--theme-border-primary);
  }

  .horizontal-bar {
    position: relative;
    display: flex;
    justify-content: space-around;
    width: 100%;
    min-width: max-content;
  }

  /* Horizontal line spanning all branches */
  .horizontal-bar::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background-color: var(--theme-border-primary);
    z-index: 1;
  }

  .branch-position {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 160px;
    flex-shrink: 0;
    padding-top: 0;
    margin-top: 7px;
  }

  .branch-position .arrow-down {
    position: relative;
    z-index: 2;
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-secondary);
    line-height: 1;
    margin-bottom: var(--spacing-1);
    background-color: var(--theme-bg-secondary);
    padding: 0 4px;
  }

  .branch-trigger {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-1);
    margin-bottom: var(--spacing-2);
    min-height: 20px;
  }

  .trigger-label {
    font-family: var(--font-display);
    font-size: 8px;
    text-align: center;
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    padding: 3px 8px;
    border-radius: 2px;
    white-space: nowrap;
    line-height: 1.4;
  }

  .branch-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    z-index: 2;
  }

  /* ── Multi-Row Specific ───────────────────────────────────── */
  .multi-row-connector {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
  }

  .row-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
  }

  .connector-vertical-between-rows {
    width: 2px;
    height: var(--spacing-4);
    background-color: var(--theme-border-primary);
  }

  /* ── Mobile ───────────────────────────────────────────────── */
  @media (max-width: 768px) {
    .evolution-tree {
      padding: var(--spacing-4);
    }

    .branch-position {
      width: 120px;
    }

    .trigger-label {
      font-size: 7px;
      padding: 2px 6px;
    }
  }
</style>
