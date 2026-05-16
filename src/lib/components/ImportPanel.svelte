<script lang="ts">
  import ImportButton from './ImportButton.svelte';
  import ImportForm from './ImportForm.svelte';

  interface Region {
    id: number;
    pokeapiId: number | null;
    name: string;
    slug: string;
  }

  let { regions = [] as Region[] } = $props();

  const TABS = [
    { key: 'gamedata',  label: 'Game Data' },
    { key: 'types',     label: 'Types' },
    { key: 'abilities', label: 'Abilities' },
    { key: 'moves',     label: 'Moves' },
    { key: 'items',     label: 'Items' },
    { key: 'pokemon',   label: 'Pokémon' },
    { key: 'locations', label: 'Locations' },
  ] as const;

  type TabKey = (typeof TABS)[number]['key'];

  let activeTab = $state<TabKey>('gamedata');
  let mode = $state<string>('all');

  function switchTab(key: TabKey) {
    activeTab = key;
    mode = key === 'pokemon' ? 'id' : 'all';
  }

  const DESCS: Record<TabKey, string> = {
    gamedata:  'Core game metadata. Dependency order: Languages → Regions → Generations → Version Groups → Versions. "Import All" runs the full sequence safely.',
    types:     'Pokémon types and their damage relations.',
    abilities: 'All passive abilities and their effects.',
    moves:     'All moves — power, accuracy, effects, and learn data.',
    items:     'Held items, key items, TMs, and consumables. No region filter (PokeAPI does not map items to generations).',
    pokemon:   'Import Pokémon with full dependency resolution (types, abilities, moves, evolution chain). Run Regional Forms and Dex Order after.',
    locations: 'Locations, areas, and wild Pokémon encounter tables. Pokémon must be imported first.',
  };

  let regionOptions = $derived(
    regions
      .filter(r => r.pokeapiId !== null)
      .map(r => ({ value: r.pokeapiId as number, label: r.name.toUpperCase() }))
  );

  let regionField = $derived([{
    name: 'regionPokeapiId',
    type: 'select' as const,
    label: 'REGION:',
    options: regionOptions,
  }]);
</script>

<div class="panel">
  <!-- Tab navigation -->
  <div class="tab-nav">
    {#each TABS as tab}
      <button
        class="tab-btn"
        class:active={activeTab === tab.key}
        onclick={() => switchTab(tab.key)}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Tab body -->
  <div class="tab-body">
    <p class="desc">{DESCS[activeTab]}</p>

    <!-- ── Game Data ──────────────────────────────────────────────────────── -->
    {#if activeTab === 'gamedata'}
      <ImportButton action="?/importGameData" label="Import All Game Data" />

      <div class="sub-section">
        <span class="sub-label">OR IMPORT INDIVIDUALLY:</span>
        <div class="btn-group">
          <ImportButton action="?/importLanguages"       label="Languages" />
          <ImportButton action="?/importRegions"         label="Regions" />
          <ImportButton action="?/importGenerations"     label="Generations" />
          <ImportButton action="?/importVersionGroups"   label="Version Groups" />
          <ImportButton action="?/importVersions"        label="Versions" />
          <ImportButton action="?/importNatures"         label="Natures" />
          <ImportButton action="?/importEggGroups"       label="Egg Groups" />
          <ImportButton action="?/importMoveLearnMethods" label="Move Learn Methods" />
          <ImportButton action="?/importEncounterMethods" label="Encounter Methods" />
        </div>
      </div>

    <!-- ── Types ─────────────────────────────────────────────────────────── -->
    {:else if activeTab === 'types'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'all'}    onclick={() => mode = 'all'}>ALL</button>
        <button class="mode-btn" class:active={mode === 'id'}     onclick={() => mode = 'id'}>ID</button>
        <button class="mode-btn" class:active={mode === 'range'}  onclick={() => mode = 'range'}>RANGE</button>
        <button class="mode-btn" class:active={mode === 'region'} onclick={() => mode = 'region'}>BY REGION</button>
      </div>
      {#if mode === 'all'}
        <ImportButton action="?/importTypes" label="Import All Types" />
      {:else if mode === 'id'}
        <ImportForm action="?/importSingleType" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '1', min: 1, label: 'ID:' }]} submitLabel="IMPORT" />
      {:else if mode === 'range'}
        <ImportForm action="?/importRangeTypes" fields={[{ name: 'fromId', type: 'number', placeholder: '1', min: 1, label: 'FROM:' }, { name: 'toId', type: 'number', placeholder: '18', min: 1, label: 'TO:' }]} submitLabel="IMPORT RANGE" />
      {:else}
        {#if regionOptions.length === 0}
          <p class="hint">No regions in database. Import Game Data first.</p>
        {:else}
          <ImportForm action="?/importTypesByRegion" fields={regionField} submitLabel="IMPORT BY REGION" />
        {/if}
      {/if}

    <!-- ── Abilities ──────────────────────────────────────────────────────── -->
    {:else if activeTab === 'abilities'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'all'}    onclick={() => mode = 'all'}>ALL</button>
        <button class="mode-btn" class:active={mode === 'id'}     onclick={() => mode = 'id'}>ID</button>
        <button class="mode-btn" class:active={mode === 'range'}  onclick={() => mode = 'range'}>RANGE</button>
        <button class="mode-btn" class:active={mode === 'region'} onclick={() => mode = 'region'}>BY REGION</button>
      </div>
      {#if mode === 'all'}
        <ImportButton action="?/importAbilities" label="Import All Abilities" />
      {:else if mode === 'id'}
        <ImportForm action="?/importSingleAbility" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '1', min: 1, label: 'ID:' }]} submitLabel="IMPORT" />
      {:else if mode === 'range'}
        <ImportForm action="?/importRangeAbilities" fields={[{ name: 'fromId', type: 'number', placeholder: '1', min: 1, label: 'FROM:' }, { name: 'toId', type: 'number', placeholder: '307', min: 1, label: 'TO:' }]} submitLabel="IMPORT RANGE" />
      {:else}
        {#if regionOptions.length === 0}
          <p class="hint">No regions in database. Import Game Data first.</p>
        {:else}
          <ImportForm action="?/importAbilitiesByRegion" fields={regionField} submitLabel="IMPORT BY REGION" />
        {/if}
      {/if}

    <!-- ── Moves ─────────────────────────────────────────────────────────── -->
    {:else if activeTab === 'moves'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'all'}    onclick={() => mode = 'all'}>ALL</button>
        <button class="mode-btn" class:active={mode === 'id'}     onclick={() => mode = 'id'}>ID</button>
        <button class="mode-btn" class:active={mode === 'range'}  onclick={() => mode = 'range'}>RANGE</button>
        <button class="mode-btn" class:active={mode === 'region'} onclick={() => mode = 'region'}>BY REGION</button>
      </div>
      {#if mode === 'all'}
        <ImportButton action="?/importMoves" label="Import All Moves" />
      {:else if mode === 'id'}
        <ImportForm action="?/importSingleMove" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '1', min: 1, label: 'ID:' }]} submitLabel="IMPORT" />
      {:else if mode === 'range'}
        <ImportForm action="?/importRangeMoves" fields={[{ name: 'fromId', type: 'number', placeholder: '1', min: 1, label: 'FROM:' }, { name: 'toId', type: 'number', placeholder: '919', min: 1, label: 'TO:' }]} submitLabel="IMPORT RANGE" />
      {:else}
        {#if regionOptions.length === 0}
          <p class="hint">No regions in database. Import Game Data first.</p>
        {:else}
          <ImportForm action="?/importMovesByRegion" fields={regionField} submitLabel="IMPORT BY REGION" />
        {/if}
      {/if}

    <!-- ── Items ─────────────────────────────────────────────────────────── -->
    {:else if activeTab === 'items'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'all'}   onclick={() => mode = 'all'}>ALL</button>
        <button class="mode-btn" class:active={mode === 'id'}    onclick={() => mode = 'id'}>ID</button>
        <button class="mode-btn" class:active={mode === 'range'} onclick={() => mode = 'range'}>RANGE</button>
      </div>
      {#if mode === 'all'}
        <ImportButton action="?/importItems" label="Import All Items" />
      {:else if mode === 'id'}
        <ImportForm action="?/importSingleItem" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '1', min: 1, label: 'ID:' }]} submitLabel="IMPORT" />
      {:else}
        <ImportForm action="?/importRangeItems" fields={[{ name: 'fromId', type: 'number', placeholder: '1', min: 1, label: 'FROM:' }, { name: 'toId', type: 'number', placeholder: '2110', min: 1, label: 'TO:' }]} submitLabel="IMPORT RANGE" />
      {/if}

    <!-- ── Pokémon ────────────────────────────────────────────────────────── -->
    {:else if activeTab === 'pokemon'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'id'}     onclick={() => mode = 'id'}>ID</button>
        <button class="mode-btn" class:active={mode === 'range'}  onclick={() => mode = 'range'}>RANGE</button>
        <button class="mode-btn" class:active={mode === 'region'} onclick={() => mode = 'region'}>BY REGION</button>
      </div>
      {#if mode === 'id'}
        <ImportForm action="?/importSinglePokemon" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '25', min: 1, label: 'ID:' }]} submitLabel="IMPORT" />
      {:else if mode === 'range'}
        <ImportForm action="?/importRangePokemon" fields={[{ name: 'fromId', type: 'number', placeholder: '1', min: 1, label: 'FROM:' }, { name: 'toId', type: 'number', placeholder: '151', min: 1, label: 'TO:' }]} submitLabel="IMPORT RANGE" />
      {:else}
        {#if regionOptions.length === 0}
          <p class="hint">No regions in database. Import Game Data first.</p>
        {:else}
          <ImportForm action="?/importPokemonByRegion" fields={regionField} submitLabel="IMPORT BY REGION" />
        {/if}
      {/if}

      <div class="extras">
        <span class="extras-label">UTILITIES:</span>
        <ImportButton action="?/importRegionalForms" label="Regional Forms" />
        <ImportButton action="?/computeDexOrder" label="Compute Dex Order" />
      </div>

    <!-- ── Locations ─────────────────────────────────────────────────────── -->
    {:else if activeTab === 'locations'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'all'}    onclick={() => mode = 'all'}>ALL</button>
        <button class="mode-btn" class:active={mode === 'region'} onclick={() => mode = 'region'}>BY REGION</button>
      </div>
      {#if mode === 'all'}
        <ImportButton action="?/importLocations" label="Import All Locations" />
      {:else}
        {#if regionOptions.length === 0}
          <p class="hint">No regions in database. Import Game Data first.</p>
        {:else}
          <ImportForm action="?/importLocationsByRegion" fields={regionField} submitLabel="IMPORT BY REGION" />
        {/if}
      {/if}
    {/if}
  </div>
</div>

<style>
  .panel {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
  }

  /* ── Tab navigation ── */

  .tab-nav {
    display: flex;
    flex-wrap: wrap;
    border-bottom: 3px solid var(--theme-border-primary);
    background-color: var(--theme-bg-tertiary);
  }

  .tab-btn {
    padding: var(--spacing-3) var(--spacing-4);
    background: none;
    border: none;
    border-right: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-inverse);
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.1s ease;
  }

  .tab-btn:last-child { border-right: none; }
  .tab-btn:hover { opacity: 0.85; }

  .tab-btn.active {
    opacity: 1;
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    border-bottom: 3px solid var(--theme-bg-secondary);
    margin-bottom: -3px;
  }

  /* ── Tab body ── */

  .tab-body {
    padding: var(--spacing-5);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .desc {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0;
  }

  /* ── Game Data sub-section ── */

  .sub-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
    padding-top: var(--spacing-3);
    border-top: 2px solid var(--theme-border-primary);
  }

  .sub-label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-secondary);
  }

  .btn-group {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }

  /* ── Mode bar ── */

  .mode-bar {
    display: flex;
    gap: var(--spacing-2);
    flex-wrap: wrap;
  }

  .mode-btn {
    padding: var(--spacing-1) var(--spacing-3);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-secondary);
    cursor: pointer;
    transition: all 0.05s ease;
  }

  .mode-btn:hover { color: var(--theme-text-primary); }

  .mode-btn.active {
    background-color: var(--theme-border-primary);
    color: var(--theme-bg-primary);
  }

  /* ── Hint ── */

  .hint {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-muted);
    margin: 0;
  }

  /* ── Pokémon extras ── */

  .extras {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    padding-top: var(--spacing-3);
    border-top: 2px solid var(--theme-border-primary);
    flex-wrap: wrap;
  }

  .extras-label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-secondary);
  }
</style>
