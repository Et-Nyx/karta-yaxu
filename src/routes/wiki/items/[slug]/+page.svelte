<script lang="ts">
  import InfoBox from '$lib/components/InfoBox.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import EffectText from '$lib/components/EffectText.svelte';
  import DataList from '$lib/components/DataList.svelte';

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

  let dataItems = $derived(() => {
    const rows: { label: string; value: string }[] = [];
    if (item.cost !== null && item.cost !== undefined) rows.push({ label: 'BUY PRICE', value: `₽${item.cost}` });
    if (item.flingPower !== null && item.flingPower !== undefined) rows.push({ label: 'FLING POWER', value: String(item.flingPower) });
    if (item.category) rows.push({ label: 'CATEGORY', value: item.category.replace(/-/g, ' ').toUpperCase() });
    return rows;
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
      <InfoBox title="DATA">
        <DataList items={dataItems()} />
      </InfoBox>

      {#if versions().length > 0}
        <VersionSelector
          bind:value={selectedVersionId}
          options={versions().map(v => ({ value: v.versionGroupId, label: v.versionGroupName.toUpperCase().replace(/-/g, ' ') }))}
        />
        {#if currentFlavorText()}
          <FlavorTextBox text={currentFlavorText()!} />
        {/if}
      {/if}
    </div>

    <!-- Right: effect -->
    {#if data.effect}
      <div class="right-col">
        <InfoBox title="EFFECT">
          <EffectText shortEffect={data.effect.shortEffect} effect={data.effect.effect} />
        </InfoBox>
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
    box-shadow: var(--shadow-card);
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
    min-width: 0;
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

  .left-col, .right-col {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  @media (max-width: 768px) {
    .detail-header {
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: var(--spacing-3);
    }

    .detail-grid {
      grid-template-columns: minmax(0, 1fr);
    }

    .left-col, .right-col { min-width: 0; }
  }
</style>
