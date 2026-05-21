<script lang="ts">
  let {
    value = $bindable(),
    options = [],
    id = '',
    name = '',
    class: className = '',
    variant = 'default',
    onchange
  }: {
    value: any;
    options: Array<{ value: any; label: string } | string>;
    id?: string;
    name?: string;
    class?: string;
    variant?: 'default' | 'header' | 'editor';
    onchange?: (val: any) => void;
  } = $props();

  function handleChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const rawVal = target.value;
    let converted: any = rawVal;
    
    // Auto-convert to number if existing value type is numeric
    if (typeof value === 'number') {
      const parsed = Number(rawVal);
      if (!isNaN(parsed)) {
        converted = parsed;
      }
    }
    
    value = converted;
    if (onchange) {
      onchange(converted);
    }
  }
</script>

<div class="retro-select-wrapper {className} {variant}">
  <select {id} {name} value={value !== null && value !== undefined ? value.toString() : ''} onchange={handleChange} class="retro-select">
    {#each options as opt}
      {#if typeof opt === 'string'}
        <option value={opt}>{opt}</option>
      {:else}
        <option value={opt.value !== null && opt.value !== undefined ? opt.value.toString() : ''}>{opt.label}</option>
      {/if}
    {/each}
  </select>
  <span class="retro-caret">▼</span>
</div>

<style>
  .retro-select-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    min-width: 80px;
    height: 32px;
  }

  .retro-select {
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    font-family: var(--font-display);
    font-size: 10px;
    padding: 6px 24px 6px 10px;
    border: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    cursor: pointer;
    line-height: 1.2;
    box-shadow: 2px 2px 0 var(--theme-border-primary);
    border-radius: 0;
  }

  .retro-select:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .retro-caret {
    position: absolute;
    right: 10px;
    pointer-events: none;
    font-size: 8px;
    color: var(--theme-text-primary);
    line-height: 1;
  }

  /* Header Variant */
  .retro-select-wrapper.header {
    min-width: 140px;
  }

  .retro-select-wrapper.header .retro-select {
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    border-color: var(--theme-text-inverse);
    box-shadow: none;
  }

  .retro-select-wrapper.header .retro-select:focus {
    border-color: var(--theme-text-accent);
  }

  .retro-select-wrapper.header .retro-caret {
    color: var(--theme-text-inverse);
  }

  /* Editor Green Variant */
  .retro-select-wrapper.editor .retro-select {
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    border-color: var(--theme-border-primary);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
  }

  /* Option native styling override */
  .retro-select :global(option) {
    font-family: var(--font-display);
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
  }
</style>
