<script lang="ts">
  import ImportButton from './ImportButton.svelte';
  import ImportForm from './ImportForm.svelte';
  import Tabs from './Tabs.svelte';

  interface Region {
    id: number;
    pokeapiId: number | null;
    name: string;
    slug: string;
  }

  interface VersionGroup {
    id: number;
    pokeapiId: number;
    name: string;
    order: number | null;
  }

  let { regions = [] as Region[], versionGroups = [] as VersionGroup[], pokemonTotal = 0 } = $props();

  const TABS = [
    { key: 'gamedata',  label: 'Game Data' },
    { key: 'types',     label: 'Types' },
    { key: 'abilities', label: 'Abilities' },
    { key: 'moves',     label: 'Moves' },
    { key: 'items',     label: 'Items' },
    { key: 'pokemon',   label: 'Pokémon' },
    { key: 'pokedexes', label: 'Pokédexes' },
    { key: 'locations', label: 'Locations' },
  ] as const;

  type TabKey = (typeof TABS)[number]['key'];

  let activeTab = $state<TabKey>('gamedata');
  let mode = $state<string>('all');

  $effect(() => {
    mode = activeTab === 'pokemon' ? 'id' : 'all';
  });

  const DESCS: Record<TabKey, string> = {
    gamedata:  'Core game metadata. Dependency order: Languages → Regions → Generations → Version Groups → Versions. "Import All" runs the full sequence safely.',
    types:     'Pokémon types and their damage relations.',
    abilities: 'All passive abilities and their effects.',
    moves:     'All moves — power, accuracy, effects, and learn data.',
    items:     'Held items, key items, TMs, and consumables. No region filter (PokeAPI does not map items to generations).',
    pokemon:   'Import Pokémon with full dependency resolution (types, abilities, moves, evolution chain). Run Regional Forms and Dex Order after.',
    pokedexes: 'Regional and national Pokédexes with their entry lists. Import Pokémon first — entries for unimported species are silently skipped.',
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

<Tabs tabs={TABS} bind:active={activeTab}>
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
        <ImportForm action="?/importSinglePokemon" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '25', min: 1, label: 'ID:' }]} submitLabel="IMPORT" showOverwriteOption={true} />
      {:else if mode === 'range'}
        <ImportForm action="?/importRangePokemon" fields={[{ name: 'fromId', type: 'number', placeholder: '1', min: 1, label: 'FROM:' }, { name: 'toId', type: 'number', placeholder: '151', min: 1, label: 'TO:' }]} submitLabel="IMPORT RANGE" showOverwriteOption={true} />
      {:else}
        {#if regionOptions.length === 0}
          <p class="hint">No regions in database. Import Game Data first.</p>
        {:else}
          <ImportForm action="?/importPokemonByRegion" fields={regionField} submitLabel="IMPORT BY REGION" showOverwriteOption={true} />
        {/if}
      {/if}

    <!-- ── Pokédexes ─────────────────────────────────────────────────────── -->
    {:else if activeTab === 'pokedexes'}
      <div class="mode-bar">
        <button class="mode-btn" class:active={mode === 'all'}    onclick={() => mode = 'all'}>ALL</button>
        <button class="mode-btn" class:active={mode === 'id'}     onclick={() => mode = 'id'}>ID</button>
        <button class="mode-btn" class:active={mode === 'bygame'} onclick={() => mode = 'bygame'}>BY GAME</button>
      </div>
      {#if pokemonTotal === 0}
        <p class="hint warn">⚠ No Pokémon imported yet — Pokédex entries will be empty. Import Pokémon first.</p>
      {/if}
      {#if mode === 'all'}
        <ImportButton action="?/importPokedexes" label="Import All Pokédexes" />
      {:else if mode === 'id'}
        <ImportForm action="?/importSinglePokedex" fields={[{ name: 'pokeapiId', type: 'number', placeholder: '1', min: 1, label: 'ID:' }]} submitLabel="IMPORT" />
      {:else}
        {#if versionGroups.length === 0}
          <p class="hint">No version groups in database — import Game Data first.</p>
        {:else}
          <ImportForm
            action="?/importPokedexesByVersionGroup"
            fields={[{
              name: 'vgPokeapiId',
              type: 'select',
              label: 'GAME:',
              options: versionGroups.map(vg => ({ value: vg.pokeapiId, label: vg.name.toUpperCase().replace(/-/g, ' ') })),
            }]}
            submitLabel="IMPORT BY GAME"
          />
        {/if}
      {/if}

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
</Tabs>

<style>
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

  .hint.warn {
    color: var(--color-text-warning);
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
