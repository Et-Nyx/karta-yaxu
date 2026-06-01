<script lang="ts">
  import { onMount, tick } from 'svelte';

  let {
    value = $bindable(),
    options = [],
    id = '',
    name = '',
    class: className = '',
    variant = 'default',
    invalid = false,
    onchange
  }: {
    value: any;
    options: Array<{ value: any; label: string } | string>;
    id?: string;
    name?: string;
    class?: string;
    variant?: 'default' | 'header' | 'editor';
    invalid?: boolean;
    onchange?: (val: any) => void;
  } = $props();

  let isOpen = $state(false);
  let searchQuery = $state('');
  let activeIndex = $state(0);
  let containerEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let dropdownEl = $state<HTMLDivElement | null>(null);

  // Normalize options to uniform array of { value, label }
  let normalizedOptions = $derived(
    options.map(opt => {
      if (!opt) {
        return { value: '', label: '' };
      }
      if (typeof opt === 'string') {
        return { value: opt, label: opt };
      }
      return {
        value: opt.value !== null && opt.value !== undefined ? opt.value : '',
        label: opt.label || ''
      };
    })
  );

  // Filter based on search query (fuzzy substring match)
  let filteredOptions = $derived(
    normalizedOptions.filter(opt =>
      opt.label.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  // Helper to get active label for rendering when closed
  let selectedLabel = $derived(
    normalizedOptions.find(opt => opt.value === value)?.label || 
    normalizedOptions.find(opt => opt.value === '')?.label || 
    '— NONE —'
  );

  function handleSelect(val: any) {
    // Auto-convert to number if existing value type is numeric
    let converted = val;
    if (typeof value === 'number') {
      const parsed = Number(val);
      if (!isNaN(parsed)) {
        converted = parsed;
      }
    }
    
    value = converted;
    isOpen = false;
    if (onchange) {
      onchange(converted);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!isOpen) {
      if (event.key === 'Enter' || event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        openDropdown();
      }
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (filteredOptions.length > 0) {
        activeIndex = (activeIndex + 1) % filteredOptions.length;
        scrollToActiveOption();
      }
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (filteredOptions.length > 0) {
        activeIndex = (activeIndex - 1 + filteredOptions.length) % filteredOptions.length;
        scrollToActiveOption();
      }
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (filteredOptions[activeIndex]) {
        handleSelect(filteredOptions[activeIndex].value);
      }
    } else if (event.key === 'Escape') {
      isOpen = false;
    }
  }

  async function openDropdown() {
    isOpen = true;
    searchQuery = '';
    activeIndex = 0;
    
    // Wait for the DOM to update to mount the input
    await tick();
    if (inputEl) {
      inputEl.focus();
    }
  }

  function scrollToActiveOption() {
    if (!dropdownEl) return;
    const activeEl = dropdownEl.querySelector('.retro-option.active') as HTMLElement;
    if (!activeEl) return;
    
    const dropdownHeight = dropdownEl.clientHeight;
    const activeTop = activeEl.offsetTop;
    const activeHeight = activeEl.offsetHeight;

    if (activeTop < dropdownEl.scrollTop) {
      dropdownEl.scrollTop = activeTop;
    } else if (activeTop + activeHeight > dropdownEl.scrollTop + dropdownHeight) {
      dropdownEl.scrollTop = activeTop + activeHeight - dropdownHeight;
    }
  }

  // Handle click outside to close the dropdown
  function handleClickOutside(event: MouseEvent) {
    if (containerEl && !containerEl.contains(event.target as Node)) {
      isOpen = false;
    }
  }

  onMount(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  });
</script>

<div
  bind:this={containerEl}
  class="retro-select-wrapper {className} {variant}"
  class:invalid
>
  <!-- Hidden inputs to submit standard HTML forms -->
  <input type="hidden" {id} {name} value={value !== null && value !== undefined ? value.toString() : ''} />

  {#if isOpen}
    <input
      bind:this={inputEl}
      type="text"
      class="retro-select-input"
      placeholder="Type to search..."
      bind:value={searchQuery}
      onkeydown={handleKeyDown}
    />
    <span class="retro-caret">▲</span>

    {#if filteredOptions.length > 0}
      <div bind:this={dropdownEl} class="retro-dropdown-options">
        {#each filteredOptions as opt, i}
          <button
            type="button"
            class="retro-option"
            class:active={i === activeIndex}
            class:selected={opt.value === value}
            onclick={() => handleSelect(opt.value)}
          >
            {opt.label.toUpperCase()}
          </button>
        {/each}
      </div>
    {:else}
      <div class="retro-dropdown-options empty">
        NO RESULTS
      </div>
    {/if}
  {:else}
    <button
      type="button"
      class="retro-select-trigger"
      onclick={openDropdown}
      onkeydown={handleKeyDown}
    >
      {selectedLabel.toUpperCase()}
    </button>
    <span class="retro-caret">▼</span>
  {/if}
</div>

<style>
  .retro-select-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    min-width: 140px;
    height: 32px;
  }

  .retro-select-trigger, .retro-select-input {
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
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: all 0.1s ease;
  }

  .retro-select-input {
    cursor: text;
  }

  .retro-select-trigger:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }

  .retro-select-trigger:active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(1px, 1px);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }

  .retro-select-trigger:focus, .retro-select-input:focus {
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
    transition: color 0.1s ease;
  }

  .retro-select-wrapper:hover .retro-caret {
    color: var(--theme-text-accent);
  }

  /* Dropdown Menu styling */
  .retro-dropdown-options {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 4px 4px 0 var(--theme-border-primary);
    max-height: 200px;
    overflow-y: auto;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    padding: 0;
  }

  /* Custom Pixel Scrollbar */
  .retro-dropdown-options::-webkit-scrollbar {
    width: 8px;
  }
  .retro-dropdown-options::-webkit-scrollbar-track {
    background: var(--theme-bg-tertiary);
  }
  .retro-dropdown-options::-webkit-scrollbar-thumb {
    background: var(--theme-text-secondary);
    border: 2px solid var(--theme-bg-tertiary);
  }

  .retro-dropdown-options.empty {
    padding: 8px 10px;
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-muted);
    text-align: center;
  }

  .retro-option {
    display: flex;
    align-items: center;
    background: none;
    border: none;
    border-radius: 0;
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    padding: 6px 12px;
    min-height: 32px;
    line-height: 1.4;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: none;
  }

  .retro-option:hover, .retro-option.active {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
    outline: none;
  }

  .retro-option.selected {
    border-left: 4px solid var(--theme-text-accent);
    padding-left: 6px;
  }

  /* Header Variant */
  .retro-select-wrapper.header {
    min-width: 140px;
  }

  .retro-select-wrapper.header .retro-select-trigger,
  .retro-select-wrapper.header .retro-select-input {
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    border-color: var(--theme-text-inverse);
    box-shadow: none;
  }

  .retro-select-wrapper.header .retro-select-trigger:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
    border-color: var(--theme-text-accent);
    transform: none;
    box-shadow: none;
  }

  .retro-select-wrapper.header .retro-select-trigger:active {
    transform: none;
    box-shadow: none;
  }

  .retro-select-wrapper.header .retro-select-trigger:focus,
  .retro-select-wrapper.header .retro-select-input:focus {
    border-color: var(--theme-text-accent);
  }

  .retro-select-wrapper.header .retro-caret {
    color: var(--theme-text-inverse);
  }

  /* Editor Green Variant */
  .retro-select-wrapper.editor .retro-select-trigger,
  .retro-select-wrapper.editor .retro-select-input {
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    border-color: var(--theme-border-primary);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
  }

  .retro-select-wrapper.editor .retro-select-trigger:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }

  .retro-select-wrapper.editor .retro-select-trigger:active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    box-shadow: 1px 1px 0 var(--theme-text-accent);
  }

  /* Invalid state (applies regardless of variant) */
  .retro-select-wrapper.invalid .retro-select-trigger,
  .retro-select-wrapper.invalid .retro-select-input,
  .retro-select-wrapper.invalid .retro-select-trigger:focus,
  .retro-select-wrapper.invalid .retro-select-input:focus {
    border-color: var(--color-text-error);
    box-shadow: 2px 2px 0 var(--color-text-error);
  }

  .retro-select-wrapper.invalid .retro-caret {
    color: var(--color-text-error);
  }
</style>
