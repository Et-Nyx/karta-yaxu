<script lang="ts">
  let {
    value = $bindable(''),
    placeholder = 'Search...',
    count,
    total,
  }: {
    value?: string;
    placeholder?: string;
    count: number;
    total?: number;
  } = $props();
</script>

<div class="search-bar">
  <input
    type="text"
    bind:value
    {placeholder}
    class="search-input"
  />
  <div class="results-count">
    {#if total !== undefined}
      {count} / {total}
    {:else}
      {count} RESULTS
    {/if}
  </div>
</div>

<style>
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
    white-space: nowrap;
  }
</style>
