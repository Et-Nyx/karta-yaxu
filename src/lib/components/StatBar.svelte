<script lang="ts">
  let {
    name = 'hp',
    value = $bindable(0),
    max = 255,
    editable = false
  }: {
    name?: string;
    value: number;
    max?: number;
    editable?: boolean;
  } = $props();

  let percentage = $derived(Math.min(100, (value / max) * 100));

  let isDragging = $state(false);
  let containerEl = $state<HTMLDivElement | null>(null);

  function handlePointerDown(e: PointerEvent) {
    if (!editable || !containerEl) return;
    isDragging = true;
    containerEl.setPointerCapture(e.pointerId);
    updateValFromEvent(e);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    updateValFromEvent(e);
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    if (containerEl) {
      containerEl.releasePointerCapture(e.pointerId);
    }
  }

  function updateValFromEvent(e: PointerEvent) {
    if (!containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    const relativeX = e.clientX - rect.left;
    const pct = Math.max(0, Math.min(1, relativeX / rect.width));
    value = Math.max(0, Math.min(max, Math.round(pct * max)));
  }
</script>

<div class="stat-row">
  <div class="stat-label">{name.replace('special-', 'sp. ').toUpperCase()}</div>
  {#if editable}
    <input
      type="number"
      min="0"
      max={max}
      class="stat-input"
      bind:value={value}
      oninput={(e) => {
        const val = Math.max(0, Math.min(max, Number(e.currentTarget.value)));
        value = val;
      }}
    />
  {:else}
    <div class="stat-value">{value.toString().padStart(3, '0')}</div>
  {/if}
  <div
    bind:this={containerEl}
    class="stat-bar-container"
    class:editable
    class:dragging={isDragging}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
  >
    <div class="stat-bar-fill" style="width: {percentage}%;"></div>
  </div>
</div>

<style>
  .stat-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    margin-bottom: var(--spacing-2);
  }

  .stat-label {
    width: 80px;
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    text-align: right;
  }

  .stat-value {
    width: 30px;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: bold;
    color: var(--theme-text-primary);
  }

  .stat-input {
    width: 44px;
    height: 24px;
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
    padding: 0;
    -moz-appearance: textfield;
  }

  .stat-input::-webkit-outer-spin-button,
  .stat-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .stat-input:focus {
    border-color: var(--theme-text-accent);
  }

  .stat-bar-container {
    flex: 1;
    height: 12px;
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card-subtle);
    transition: border-color 0.1s ease, box-shadow 0.1s ease;
  }

  .stat-bar-container.editable {
    cursor: ew-resize;
    user-select: none;
    touch-action: none;
  }

  .stat-bar-container.editable:hover,
  .stat-bar-container.editable.dragging {
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0 var(--theme-text-accent);
  }

  .stat-bar-fill {
    height: 100%;
    background-color: var(--theme-text-accent);
    border-right: 1px solid var(--theme-border-primary);
  }
</style>
