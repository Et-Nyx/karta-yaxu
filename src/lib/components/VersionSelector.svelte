<script lang="ts">
  let {
    options,
    value = $bindable(),
  }: { options: { value: number; label: string }[]; value: number | null } = $props();
</script>

<div class="version-selector-container">
  <label for="versionSelect">VERSION:</label>
  <select id="versionSelect" bind:value class="version-select">
    {#each options as opt}
      <option value={opt.value}>{opt.label}</option>
    {/each}
  </select>
</div>

<style>
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
    white-space: nowrap;
  }

  .version-select {
    flex: 1;
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 10px;
    min-width: 0;
  }

  @media (max-width: 768px) {
    .version-selector-container { flex-wrap: wrap; width: 100%; box-sizing: border-box; }
    .version-select { max-width: 100%; }
  }
</style>
