<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    type = 'button',
    variant = 'primary', // 'primary' (large), 'secondary' (small), 'text' (inline link)
    disabled = false,
    onclick = null,
    children,
    class: customClass = '',
    ...restProps
  }: {
    type?: 'button' | 'submit' | 'reset';
    variant?: 'primary' | 'secondary' | 'text';
    disabled?: boolean;
    onclick?: ((e: MouseEvent) => void) | null;
    children?: Snippet;
    class?: string;
    [key: string]: any;
  } = $props();
</script>

<button
  {type}
  {disabled}
  onclick={onclick}
  class="retro-btn btn-{variant} {customClass}"
  {...restProps}
>
  {#if children}
    {@render children()}
  {/if}
</button>

<style>
  .retro-btn {
    font-family: var(--font-display);
    cursor: pointer;
    transition: all 0.1s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    outline: none;
    border-radius: 0;
    text-transform: uppercase;
    text-decoration: none;
  }

  .retro-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  /* Variant: primary (matches .save-btn / standard buttons) */
  .btn-primary {
    font-size: 10px;
    color: var(--theme-text-primary);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2) var(--spacing-4);
    box-shadow: var(--shadow-card-subtle);
  }

  .btn-primary:hover:not(:disabled) {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }

  .btn-primary:active:not(:disabled) {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }

  /* Variant: secondary (matches .save-btn-sm / small action buttons) */
  .btn-secondary {
    font-size: 9px;
    color: var(--theme-text-primary);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    padding: 2px var(--spacing-3);
    box-shadow: var(--shadow-card-subtle);
  }

  .btn-secondary:hover:not(:disabled) {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 2px 2px 0 var(--theme-text-accent);
  }

  .btn-secondary:active:not(:disabled) {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }

  /* Variant: text (matches .cancel-link) */
  .btn-text {
    background: none;
    border: none;
    font-size: 9px;
    color: var(--theme-text-inverse);
    opacity: 0.85;
    padding: 0;
  }

  .btn-text:hover:not(:disabled) {
    opacity: 1;
    color: var(--theme-text-accent);
  }
</style>
