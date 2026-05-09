<script lang="ts">
  import ImportStatusGrid from '$lib/components/ImportStatusGrid.svelte';
  import ImportButton from '$lib/components/ImportButton.svelte';
  import ImportLog from '$lib/components/ImportLog.svelte';
  
  import { enhance } from '$app/forms';
  import { onMount } from 'svelte';
  
  let { data } = $props();

  let liveLogs = $state(data.logs);

  onMount(() => {
    const eventSource = new EventSource('/api/import/sse');
    
    eventSource.onmessage = (event) => {
      try {
        const newLog = JSON.parse(event.data);
        // Ensure the mock id is unique for Svelte #each keys if ImportLog uses id
        newLog.id = Math.random(); 
        liveLogs = [newLog, ...liveLogs].slice(0, 50); // Keep latest 50
      } catch (e) {
        console.error('SSE parsing error', e);
      }
    };

    return () => {
      eventSource.close();
    };
  });
</script>

<svelte:head>
  <title>Import | Karta Yaxu</title>
</svelte:head>

<div class="import-page">
  <div class="page-header">
    <h2>PokeAPI Import Dashboard</h2>
    <p>Database seeded on-demand. Note that manual edits are protected from overwriting.</p>
  </div>

  <section class="status-section">
    <ImportStatusGrid status={data.status} />
  </section>

  <div class="actions-grid">
    <!-- Game Data -->
    <section class="action-panel">
      <h3>1. Game Metadata</h3>
      <p>Imports Languages, Regions, Generations, Versions, and Pokedexes.</p>
      <ImportButton action="?/importGameData" label="Import All Game Data" />
    </section>

    <!-- Core Data -->
    <section class="action-panel">
      <h3>2. Core Data</h3>
      <p>Imports individual core data collections.</p>
      <div class="btn-group">
        <ImportButton action="?/importTypes" label="Types" />
        <ImportButton action="?/importAbilities" label="Abilities" />
        <ImportButton action="?/importMoves" label="Moves" />
        <ImportButton action="?/importItems" label="Items" />
      </div>
    </section>

    <!-- Pokemon -->
    <section class="action-panel">
      <h3>3. Pokémon</h3>
      <p>Import Pokémon with full dependency resolution (types, abilities, moves, evolution chain).</p>
      
      <div class="form-row">
        <form method="POST" action="?/importSinglePokemon" class="inline-form" use:enhance>
          <label for="pokeapiId">Single ID:</label>
          <input type="number" id="pokeapiId" name="pokeapiId" placeholder="25" required min="1" />
          <button type="submit" class="submit-btn">Import Single</button>
        </form>
      </div>

      <div class="form-row">
        <form method="POST" action="?/importRangePokemon" class="inline-form" use:enhance>
          <label>Range:</label>
          <input type="number" name="fromId" placeholder="1" required min="1" />
          <span>to</span>
          <input type="number" name="toId" placeholder="151" required min="1" />
          <button type="submit" class="submit-btn">Import Range</button>
        </form>
      </div>
    </section>
  </div>

  <section class="log-section">
    <ImportLog logs={liveLogs} />
  </section>
</div>

<style>
  .import-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
  }

  .page-header p {
    color: var(--theme-text-secondary);
    font-size: 14px;
    margin: 0;
  }

  .actions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: var(--spacing-4);
  }

  .action-panel {
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .action-panel h3 {
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
    font-size: 14px;
  }

  .action-panel p {
    color: var(--theme-text-secondary);
    font-size: 12px;
    margin-bottom: var(--spacing-4);
  }

  .btn-group {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-3);
  }

  .form-row {
    margin-bottom: var(--spacing-3);
  }

  .inline-form {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }

  .inline-form label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    width: 70px;
  }

  .inline-form span {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
  }

  input[type="number"] {
    width: 60px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .submit-btn {
    padding: var(--spacing-2) var(--spacing-3);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    cursor: pointer;
    white-space: nowrap;
  }

  .submit-btn:hover {
    background-color: var(--theme-bg-hover);
  }

  .log-section {
    margin-top: var(--spacing-4);
  }
</style>
