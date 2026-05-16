<script lang="ts">
  let { data } = $props();
  let item = $derived(data.item);

  // Unique version groups that have a flavor text, preserving order
  let versions = $derived(() => {
    const seen = new Set<number>();
    return data.flavorTexts.filter(f => {
      if (seen.has(f.versionGroupId)) return false;
      seen.add(f.versionGroupId);
      return true;
    });
  });

  let selectedVersionId = $state<number | null>(null);

  $effect(() => {
    const vs = versions();
    if (selectedVersionId === null && vs.length > 0) {
      const frlg = vs.find(v => v.versionGroupName === 'firered-leafgreen');
      selectedVersionId = (frlg ?? vs[vs.length - 1]).versionGroupId;
    }
  });

  let currentFlavorText = $derived(() => {
    if (!selectedVersionId) return null;
    return data.flavorTexts.find(f => f.versionGroupId === selectedVersionId)?.flavorText ?? null;
  });
</script>

<svelte:head>
  <title>{item.name.replace(/-/g, ' ').toUpperCase()} | Karta Yaxu</title>
</svelte:head>

<div class="item-detail">
  <header class="detail-header">
    {#if item.spriteUrl}
      <img src={item.spriteUrl} alt={item.name} class="header-sprite" style="image-rendering: pixelated;" />
    {/if}
    <div class="header-info">
      <h1>{item.name.replace(/-/g, ' ').toUpperCase()}</h1>
      {#if item.category}
        <span class="category-badge">{item.category.replace(/-/g, ' ')}</span>
      {/if}
    </div>
  </header>

  <div class="detail-grid">
    <!-- Left: data + flavor text -->
    <div class="left-col">
      <div class="info-box">
        <h3>DATA</h3>
        <ul class="data-list">
          {#if item.cost !== null && item.cost !== undefined}
            <li><span class="label">BUY PRICE</span><span class="value">₽{item.cost}</span></li>
          {/if}
          {#if item.flingPower !== null && item.flingPower !== undefined}
            <li><span class="label">FLING POWER</span><span class="value">{item.flingPower}</span></li>
          {/if}
          {#if item.category}
            <li><span class="label">CATEGORY</span><span class="value">{item.category.replace(/-/g, ' ').toUpperCase()}</span></li>
          {/if}
        </ul>
      </div>

      {#if versions().length > 0}
        <div class="version-selector-container">
          <label for="versionSelect">VERSION:</label>
          <select id="versionSelect" bind:value={selectedVersionId} class="version-select">
            {#each versions() as v}
              <option value={v.versionGroupId}>{v.versionGroupName.toUpperCase().replace(/-/g, ' ')}</option>
            {/each}
          </select>
        </div>

        {#if currentFlavorText()}
          <div class="flavor-text-box">
            <p>{currentFlavorText()}</p>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Right: effect -->
    {#if data.effect}
      <div class="right-col">
        <div class="info-box effect-box">
          <h3>EFFECT</h3>
          <p class="short-effect">{data.effect.shortEffect}</p>
          {#if data.effect.effect && data.effect.effect !== data.effect.shortEffect}
            <p class="long-effect">{data.effect.effect}</p>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .item-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  /* ── Header ── */

  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-6);
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
  }

  .header-sprite {
    width: 96px;
    height: 96px;
    object-fit: contain;
    flex-shrink: 0;
  }

  .header-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 20px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .category-badge {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    text-transform: capitalize;
  }

  /* ── Grid ── */

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
    align-items: start;
  }

  @media (max-width: 768px) {
    .detail-grid { grid-template-columns: 1fr; }
  }

  .left-col, .right-col {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  /* ── Shared box ── */

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

  /* ── Data list ── */

  .data-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .data-list li {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 14px;
  }

  .data-list .label { color: var(--theme-text-secondary); }
  .data-list .value { color: var(--theme-text-primary); font-weight: bold; }

  /* ── Version selector ── */

  .version-selector-container {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .version-selector-container label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
  }

  .version-select {
    flex: 1;
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 10px;
  }

  /* ── Flavor text ── */

  .flavor-text-box {
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.5;
    color: var(--theme-text-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .flavor-text-box p { margin: 0; }

  /* ── Effect ── */

  .short-effect {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
    line-height: 1.6;
    margin: 0 0 var(--spacing-3) 0;
  }

  .long-effect {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    line-height: 1.6;
    margin: 0;
    padding-top: var(--spacing-3);
    border-top: 1px dashed var(--theme-border-primary);
  }
</style>
