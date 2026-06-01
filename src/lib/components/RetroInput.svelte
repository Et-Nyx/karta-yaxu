<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  let {
    value = $bindable(),
    type = 'text',
    min,
    max,
    step,
    placeholder,
    disabled = false,
    name,
    invalid = false,
    class: className = '',
    oninput,
    ...rest
  }: HTMLInputAttributes & {
    value?: any;
    type?: string;
    invalid?: boolean;
  } = $props();
</script>

<input
  {type}
  {name}
  {min}
  {max}
  {step}
  {placeholder}
  {disabled}
  bind:value={value}
  class="retro-input {className}"
  class:invalid
  {oninput}
  {...rest}
/>

<style>
  .retro-input {
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: bold;
    text-align: center;
    outline: none;
    box-sizing: border-box;
    padding: 0 var(--spacing-2);
    height: 24px;
    -moz-appearance: textfield;
    transition: border-color 0.1s ease, box-shadow 0.1s ease;
  }

  .retro-input::-webkit-outer-spin-button,
  .retro-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .retro-input:focus {
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0 var(--theme-text-accent);
  }

  .retro-input.invalid,
  .retro-input.invalid:focus {
    border-color: var(--color-text-error);
    box-shadow: 2px 2px 0 var(--color-text-error);
  }

  .retro-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
