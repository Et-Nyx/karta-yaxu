<script lang="ts">
  let {
    name,
    slug,
    spriteUrl,
    category,
    cost,
  }: {
    name: string;
    slug: string;
    spriteUrl: string | null;
    category: string | null;
    cost: number | null;
  } = $props();

  let displayName = $derived(name.replace(/-/g, ' ').toUpperCase());
  let displayCategory = $derived(category ? category.replace(/-/g, ' ') : null);
</script>

<a href="/wiki/items/{slug}" class="item-card card-interactive">
  <div class="sprite-wrap">
    {#if spriteUrl}
      <img src={spriteUrl} alt={name} loading="lazy" />
    {:else}
      <div class="no-sprite">?</div>
    {/if}
  </div>

  <div class="item-info">
    <span class="item-name">{displayName}</span>
    {#if displayCategory}
      <span class="item-category">{displayCategory}</span>
    {/if}
    {#if cost !== null && cost !== undefined}
      <span class="item-cost">₽{cost}</span>
    {/if}
  </div>
</a>

<style>
  .item-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-2);
    padding: var(--spacing-3);
    text-align: center;
    min-height: 120px;
  }

  .sprite-wrap {
    width: 48px;
    height: 48px;
    display: flex;
    align-items: center;
    justify-content: center;
    image-rendering: pixelated;
  }

  .sprite-wrap img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .no-sprite {
    font-family: var(--font-display);
    font-size: 18px;
    color: var(--theme-text-muted);
  }

  .item-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
  }

  .item-name {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-primary);
    line-height: 1.4;
  }

  .item-category {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-secondary);
    text-transform: capitalize;
  }

  .item-cost {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }
</style>
