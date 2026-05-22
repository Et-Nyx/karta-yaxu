<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<div class="info-box">
  <h3>{title}</h3>
  {@render children()}
</div>

<style>
  .info-box {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-4);
  }

  .info-box h3 {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-inverse);
    margin-top: calc(-1 * var(--spacing-4));
    margin-left: calc(-1 * var(--spacing-4));
    margin-right: calc(-1 * var(--spacing-4));
    margin-bottom: var(--spacing-4);
    padding: var(--spacing-3) var(--spacing-4);
    background-color: var(--theme-bg-tertiary);
    border-bottom: 2px solid var(--theme-border-primary);
  }
</style>
