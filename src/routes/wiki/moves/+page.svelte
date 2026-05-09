<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import MoveCategoryIcon from '$lib/components/MoveCategoryIcon.svelte';
  
  let { data } = $props();

  let searchQuery = $state('');

  let filteredMoves = $derived(
    data.moves.filter(m => 
      m.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (m.typeName && m.typeName.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );
</script>

<svelte:head>
  <title>Moves | Karta Yaxu</title>
</svelte:head>

<div class="moves-page">
  <div class="page-header">
    <h2>MOVES</h2>
    <div class="search-bar">
      <input 
        type="text" 
        bind:value={searchQuery} 
        placeholder="Search by name or type... (/)" 
        class="search-input"
      />
      <div class="results-count">
        {filteredMoves.length} RESULTS
      </div>
    </div>
  </div>

  <div class="moves-list">
    {#each filteredMoves as m (m.id)}
      <a href="/wiki/moves/{m.slug}" class="move-card card-interactive">
        <div class="move-info">
          <div class="move-name">{m.name.toUpperCase()}</div>
          <MoveCategoryIcon category={m.damageClass || 'physical'} size="md" />
          {#if m.typeName}
            <TypeBadge name={m.typeName} size="sm" />
          {/if}
        </div>
        
        <div class="move-stats">
          <div class="stat">
            <span class="label">PWR</span>
            <span class="value">{m.power !== null ? m.power : '-'}</span>
          </div>
          <div class="stat">
            <span class="label">ACC</span>
            <span class="value">{m.accuracy !== null ? m.accuracy + '%' : '-'}</span>
          </div>
          <div class="stat">
            <span class="label">PP</span>
            <span class="value">{m.pp !== null ? m.pp : '-'}</span>
          </div>
        </div>
      </a>
    {/each}
  </div>
</div>

<style>
  .moves-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
  }

  .search-input {
    flex: 1;
    max-width: 400px;
    padding: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .results-count {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .moves-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .move-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-3) var(--spacing-4);
  }

  .move-info {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    flex: 1;
  }

  .move-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    font-weight: bold;
    width: 200px;
  }

  .move-stats {
    display: flex;
    gap: var(--spacing-6);
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 50px;
  }

  .stat .label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    margin-bottom: 2px;
  }

  .stat .value {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  @media (max-width: 768px) {
    .move-card {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--spacing-4);
    }
    .move-info {
      width: 100%;
      flex-wrap: wrap;
    }
    .move-stats {
      width: 100%;
      justify-content: space-between;
    }
  }
</style>
