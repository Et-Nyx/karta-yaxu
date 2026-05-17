<script lang="ts">
  import Tabs from '$lib/components/Tabs.svelte';

  let { data } = $props();
  let loc = $derived(data.location);

  function titleCase(str: string): string {
    return str.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  }

  function areaLabel(name: string): string {
    return titleCase(name);
  }

const versionTabs = data.availableVersions.map(v => ({
    key: v.slug,
    label: v.name.toUpperCase().replace(/-/g, ' '),
  }));

  let activeVersionSlug = $state(
    data.availableVersions.find(v => v.id === data.selectedVersionId)?.slug ?? ''
  );

  $effect(() => {
    const currentSlug = data.availableVersions.find(v => v.id === data.selectedVersionId)?.slug;
    if (activeVersionSlug && activeVersionSlug !== currentSlug) {
      const url = new URL(window.location.href);
      url.searchParams.set('version', activeVersionSlug);
      window.location.href = url.toString();
    }
  });

  type EncounterRow = {
    methodName: string;
    methodOrder: number | null;
    pokemonName: string;
    pokemonSlug: string;
    pokemonSprite: string | null;
    minLevel: number;
    maxLevel: number;
    chance: number;
  };

  function groupByMethod(encounters: EncounterRow[]) {
    const groups: { methodName: string; encounters: EncounterRow[] }[] = [];
    const seen = new Map<string, number>();
    for (const enc of encounters) {
      if (!seen.has(enc.methodName)) {
        seen.set(enc.methodName, groups.length);
        groups.push({ methodName: enc.methodName, encounters: [] });
      }
      groups[seen.get(enc.methodName)!].encounters.push(enc);
    }
    return groups;
  }

  // Per-area active method tab — initialised to the first method for each area
  let activeMethodByArea = $state<Record<number, string>>(
    Object.fromEntries(
      data.areas
        .filter(a => a.encounters.length > 0)
        .map(a => {
          const groups = groupByMethod(a.encounters);
          return [a.id, groups[0]?.methodName ?? ''];
        })
    )
  );
</script>

<svelte:head>
  <title>{titleCase(loc.name)} | Karta Yaxu</title>
</svelte:head>

<div class="location-detail">
  <header class="detail-header">
    <div class="header-main">
      <h1>{titleCase(loc.name)}</h1>
      {#if loc.regionName}
        <a href="/wiki/locations" class="region-link">{loc.regionName.toUpperCase()}</a>
      {/if}
    </div>
  </header>

  {#if data.availableVersions.length === 0}
    <div class="empty-state">
      <p>No encounter data available for this location.</p>
      <p class="hint">Import encounter data via the <a href="/import">Import Panel</a>.</p>
    </div>
  {:else}
    <Tabs tabs={versionTabs} bind:active={activeVersionSlug}>
      <div class="areas-list">
        {#each data.areas as area}
          {#if area.encounters.length > 0}
            {@const methodGroups = groupByMethod(area.encounters)}
            {@const areaTabs = methodGroups.map(g => ({ key: g.methodName, label: g.methodName.replace(/-/g, ' ').toUpperCase() }))}
            {@const activeGroup = methodGroups.find(g => g.methodName === activeMethodByArea[area.id])}
            <Tabs title={areaLabel(area.name)} tabs={areaTabs} bind:active={activeMethodByArea[area.id]}>
              {#if activeGroup}
                <div class="table-wrap">
                  <table class="encounter-table">
                    <thead>
                      <tr>
                        <th>POKÉMON</th>
                        <th>LEVEL</th>
                        <th>CHANCE</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each activeGroup.encounters as enc}
                        <tr>
                          <td class="pokemon-cell">
                            {#if enc.pokemonSprite}
                              <img src={enc.pokemonSprite} alt={enc.pokemonName} class="mini-sprite" style="image-rendering: pixelated;" />
                            {/if}
                            <a href="/wiki/pokemon/{enc.pokemonSlug}" class="pokemon-link">
                              {enc.pokemonName.toUpperCase()}
                            </a>
                          </td>
                          <td class="level-cell">
                            {#if enc.minLevel === enc.maxLevel}
                              {enc.minLevel}
                            {:else}
                              {enc.minLevel}–{enc.maxLevel}
                            {/if}
                          </td>
                          <td class="chance-cell">{enc.chance}%</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </Tabs>
          {/if}
        {/each}

        {#if data.areas.every(a => a.encounters.length === 0)}
          <div class="empty-state">
            <p>No encounters for this version in this location.</p>
          </div>
        {/if}
      </div>
    </Tabs>
  {/if}
</div>

<style>
  .location-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  /* ── Header ── */

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: var(--shadow-card);
  }

  .header-main {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    flex-wrap: wrap;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 20px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .region-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-secondary);
    text-decoration: none;
    border: 2px solid var(--theme-border-primary);
    padding: 3px 8px;
    transition: all 0.05s ease;
  }

  .region-link:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  /* ── Areas ── */

  .areas-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  /* ── Encounter table ── */

  .table-wrap {
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
  }

  .encounter-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 13px;
    border: 3px solid var(--theme-border-primary);
  }

  thead tr {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  th {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-inverse);
    padding: var(--spacing-2) var(--spacing-4);
    text-align: left;
    white-space: nowrap;
  }

  th:not(:first-child) { text-align: center; }

  tbody tr {
    border-bottom: 1px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
  }

  tbody tr:last-child { border-bottom: none; }
  tbody tr:hover { background-color: var(--theme-bg-primary); }

  td {
    padding: var(--spacing-2) var(--spacing-4);
    color: var(--theme-text-primary);
  }

  .pokemon-cell {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .mini-sprite {
    width: 32px;
    height: 32px;
    object-fit: contain;
    flex-shrink: 0;
  }

  .pokemon-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-primary);
    text-decoration: none;
    transition: color 0.05s ease;
  }

  .pokemon-link:hover { color: var(--theme-text-accent); }

  .level-cell, .chance-cell {
    text-align: center;
    color: var(--theme-text-secondary);
    font-size: 12px;
  }

  /* ── Empty state ── */

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-8);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint { font-size: 12px; color: var(--theme-text-muted); }
  .empty-state a { color: var(--theme-text-accent); text-decoration: none; }
</style>
