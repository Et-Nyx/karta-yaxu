<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';

  let { data } = $props();
  let loc = $derived(data.location);

  function titleCase(str: string): string {
    return str.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
  }

  function areaLabel(name: string): string {
    // Strip the location prefix from area names (e.g. "pallet-town-area" → "Area")
    return titleCase(name);
  }

  function versionLabel(name: string): string {
    return name.toUpperCase().replace(/-/g, ' ');
  }

  let selectedVersionId = $state<number | null>(data.selectedVersionId);

  function goToVersion(versionSlug: string) {
    const url = new URL(window.location.href);
    url.searchParams.set('version', versionSlug);
    window.location.href = url.toString();
  }
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
    <div class="version-selector-container">
      <label for="versionSelect">VERSION:</label>
      <select
        id="versionSelect"
        class="version-select"
        onchange={(e) => goToVersion((e.target as HTMLSelectElement).value)}
      >
        {#each data.availableVersions as v}
          <option value={v.slug} selected={v.id === data.selectedVersionId}>
            {versionLabel(v.name)}
          </option>
        {/each}
      </select>
    </div>

    <div class="areas-list">
      {#each data.areas as area}
        {#if area.encounters.length > 0}
          <div class="area-section">
            <h3 class="area-title">{areaLabel(area.name)}</h3>

            <table class="encounter-table">
              <thead>
                <tr>
                  <th>POKÉMON</th>
                  <th>METHOD</th>
                  <th>LEVEL</th>
                  <th>CHANCE</th>
                </tr>
              </thead>
              <tbody>
                {#each area.encounters as enc}
                  <tr>
                    <td class="pokemon-cell">
                      {#if enc.pokemonSprite}
                        <img src={enc.pokemonSprite} alt={enc.pokemonName} class="mini-sprite" style="image-rendering: pixelated;" />
                      {/if}
                      <a href="/wiki/pokemon/{enc.pokemonSlug}" class="pokemon-link">
                        {enc.pokemonName.toUpperCase()}
                      </a>
                    </td>
                    <td class="method-cell">{enc.methodName.replace(/-/g, ' ').toUpperCase()}</td>
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
      {/each}

      {#if data.areas.every(a => a.encounters.length === 0)}
        <div class="empty-state">
          <p>No encounters for this version in this location.</p>
        </div>
      {/if}
    </div>
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
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
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

  /* ── Version selector ── */

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
    max-width: 300px;
  }

  /* ── Areas ── */

  .areas-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .area-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .area-title {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  /* ── Encounter table ── */

  .encounter-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 13px;
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
  }

  thead tr {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 3px solid var(--theme-border-primary);
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

  .method-cell, .level-cell, .chance-cell {
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
