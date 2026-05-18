<script lang="ts">
  import type { Snippet } from 'svelte';

  let { head, children, clickable = false, compact = false }: {
    head: Snippet;
    children: Snippet;
    clickable?: boolean;
    compact?: boolean;
  } = $props();
</script>

<div class="table-wrap">
  <table class="data-table" class:clickable class:compact>
    <thead>
      <tr>{@render head()}</tr>
    </thead>
    <tbody>
      {@render children()}
    </tbody>
  </table>
</div>

<style>
  .table-wrap {
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    overflow-x: auto;
    overflow-y: hidden;
    -webkit-overflow-scrolling: touch;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .data-table :global(thead tr) {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 3px solid var(--theme-border-primary);
  }

  .data-table :global(th) {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-inverse);
    padding: var(--spacing-3) var(--spacing-4);
    white-space: nowrap;
  }

  /* Lower specificity so parent pages can override text-align with a class */
  :global(.data-table th) { text-align: left; }

  .data-table :global(tbody tr) {
    border-bottom: 1px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    transition: background-color 0.05s ease;
  }

  .data-table :global(tbody tr:nth-child(even)) {
    background-color: var(--theme-bg-primary);
  }

  .data-table :global(tbody tr:last-child) {
    border-bottom: none;
  }

  .data-table :global(tbody tr:hover) {
    background-color: var(--theme-bg-hover);
  }

  /* Clickable rows — accent tint on hover + pointer cursor */
  .data-table.clickable :global(tbody tr) {
    cursor: pointer;
  }

  .data-table.clickable :global(tbody tr:hover) {
    background-color: color-mix(in srgb, var(--theme-text-accent) 12%, var(--theme-bg-hover));
  }

  .data-table :global(td) {
    padding: var(--spacing-3) var(--spacing-4);
    color: var(--theme-text-primary);
    white-space: nowrap;
  }

  .data-table.compact :global(th) {
    padding: var(--spacing-2) var(--spacing-2);
  }

  .data-table.compact :global(td) {
    padding: var(--spacing-2) var(--spacing-2);
  }
</style>
