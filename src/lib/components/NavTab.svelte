<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    href,
    active = false,
    variant = 'default',
    children,
  }: {
    href?: string;
    active?: boolean;
    variant?: 'default' | 'login';
    children?: Snippet;
  } = $props();
</script>

{#if href}
  <a {href} class="nav-tab" class:active class:login={variant === 'login'}>
    {@render children?.()}
  </a>
{:else}
  <button type="submit" class="nav-tab login">
    {@render children?.()}
  </button>
{/if}

<style>
  .nav-tab {
    font-family: var(--font-display);
    font-size: 12px;
    padding: var(--spacing-2) var(--spacing-3);
    color: var(--theme-text-inverse);
    border: 2px solid transparent;
    background: none;
    cursor: pointer;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    line-height: 1;
    flex-shrink: 0;
    transition: all 0.1s ease;
  }

  .nav-tab:hover {
    color: var(--theme-text-accent);
  }

  .nav-tab.active {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .nav-tab.login {
    border-color: var(--theme-border-primary);
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .nav-tab.login:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }

  .nav-tab.login:active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }


</style>
