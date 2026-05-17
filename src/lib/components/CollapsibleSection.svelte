<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    count = null,
    open = $bindable(true),
    children,
  }: {
    title: string;
    count?: number | null;
    open?: boolean;
    children: Snippet;
  } = $props();
</script>

<div class="collapsible-section">
  <button
    class="section-header"
    onclick={() => open = !open}
    aria-expanded={open}
  >
    <span class="section-title">{title}</span>
    <span class="section-meta">
      {#if count !== null}
        <span class="section-count">{count}</span>
      {/if}
      <span class="chevron" class:open>▶</span>
    </span>
  </button>

  {#if open}
    <div class="section-content">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .collapsible-section {
    display: flex;
    flex-direction: column;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    cursor: pointer;
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    transition: all 0.05s ease;
    margin-bottom: var(--spacing-3);
  }

  .section-header:hover {
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0px var(--theme-text-accent);
    transform: translate(-1px, -1px);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
  }

  .section-meta {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .section-count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
  }

  .chevron {
    font-size: 10px;
    color: var(--theme-text-secondary);
    transition: transform 0.15s ease;
    display: inline-block;
  }

  .chevron.open {
    transform: rotate(90deg);
  }

  .section-content {
    margin-bottom: var(--spacing-2);
  }
</style>
