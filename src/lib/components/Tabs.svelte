<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    tabs,
    active = $bindable(),
    title = null,
    children,
  }: {
    tabs: readonly { key: string; label: string }[];
    active: string;
    title?: string | null;
    children: Snippet;
  } = $props();
</script>

<div class="tabs-panel">
  {#if title}
    <div class="tabs-title">{title}</div>
  {/if}
  <div class="tab-nav">
    {#each tabs as tab}
      <button
        class="tab-btn"
        class:active={active === tab.key}
        onclick={() => active = tab.key}
      >
        {tab.label}
      </button>
    {/each}
  </div>
  <div class="tab-body">
    {@render children()}
  </div>
</div>

<style>
  .tabs-panel {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
  }

  .tabs-title {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    padding: var(--spacing-3) var(--spacing-4);
    border-bottom: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
  }

  .tab-nav {
    display: flex;
    flex-wrap: wrap;
    border-bottom: 3px solid var(--theme-border-primary);
    background-color: var(--theme-bg-tertiary);
  }

  .tab-btn {
    padding: var(--spacing-3) var(--spacing-4);
    background: none;
    border: none;
    border-right: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-inverse);
    cursor: pointer;
    opacity: 0.6;
    transition: all 0.1s ease;
  }

  .tab-btn:last-child { border-right: none; }
  .tab-btn:hover {
    opacity: 1;
    color: var(--theme-text-accent);
  }

  .tab-btn.active {
    opacity: 1;
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    border-bottom: 3px solid var(--theme-bg-secondary);
    margin-bottom: -3px;
  }

  .tab-body {
    padding: var(--spacing-5);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }
</style>
