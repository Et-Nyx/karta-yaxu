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
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-4) 0;
    border-bottom: 2px solid var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
  }
</style>
