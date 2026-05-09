<script lang="ts">
  // Tipos para o status de importação
  export type SourceCounts = {
    total: number;
    pokeapi: number;
    edited: number;
    custom: number;
  };

  export type ImportStatus = Record<string, SourceCounts>;

  let { status = {} as ImportStatus } = $props();

  const ENTITIES = [
    { key: 'languages', label: 'Languages', max: 13 },
    { key: 'regions', label: 'Regions', max: 10 },
    { key: 'generations', label: 'Generations', max: 9 },
    { key: 'types', label: 'Types', max: 18 },
    { key: 'abilities', label: 'Abilities', max: 307 },
    { key: 'moves', label: 'Moves', max: 919 },
    { key: 'items', label: 'Items', max: 2110 },
    { key: 'pokemon', label: 'Pokémon', max: 1025 },
  ];

  function getPercentage(count: number, max: number): number {
    if (!max || max === 0) return 0;
    return Math.min(100, Math.round((count / max) * 100));
  }
</script>

<div class="status-grid">
  {#each ENTITIES as entity}
    {@const counts = status[entity.key] || { total: 0, pokeapi: 0, edited: 0, custom: 0 }}
    {@const pct = getPercentage(counts.total, entity.max)}
    
    <div class="status-card">
      <div class="card-header">
        <span class="entity-name">{entity.label}</span>
        <span class="entity-count">{counts.total} / {entity.max}</span>
      </div>
      
      <!-- Barra de progresso total -->
      <div class="progress-track">
        <div class="progress-fill" style="width: {pct}%"></div>
      </div>
      
      <!-- Breakdown por source -->
      <div class="source-breakdown">
        <div class="source-item">
          <span class="dot pokeapi"></span>
          <span>API: {counts.pokeapi}</span>
        </div>
        <div class="source-item">
          <span class="dot edited"></span>
          <span>Edit: {counts.edited}</span>
        </div>
        <div class="source-item">
          <span class="dot custom"></span>
          <span>Cust: {counts.custom}</span>
        </div>
      </div>
    </div>
  {/each}
</div>

<style>
  .status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--spacing-4);
    margin-bottom: var(--spacing-6);
  }

  .status-card {
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    padding: var(--spacing-3);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--spacing-2);
  }

  .entity-name {
    font-family: var(--font-display);
    font-size: 10px;
    font-weight: bold;
    color: var(--theme-text-primary);
  }

  .entity-count {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
  }

  .progress-track {
    width: 100%;
    height: 8px;
    background-color: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    margin-bottom: var(--spacing-2);
  }

  .progress-fill {
    height: 100%;
    background-color: var(--theme-text-primary);
  }

  .source-breakdown {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .dot.pokeapi { background-color: var(--theme-text-primary); }
  .dot.edited { background-color: #f1fa8c; }
  .dot.custom { background-color: #8be9fd; }
</style>
