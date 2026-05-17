<script lang="ts">
  let {
    name,
    slug,
    regionName = null,
  }: {
    name: string;
    slug: string;
    regionName?: string | null;
  } = $props();

  let displayName = $derived(
    name.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
  );
</script>

<a href="/wiki/locations/{slug}" class="location-card card-interactive">
  <span class="card-content">
    <span class="name">{displayName}</span>
    {#if regionName}
      <span class="region">{regionName.toUpperCase()}</span>
    {/if}
  </span>
</a>

<style>
  .location-card {
    display: flex;
    align-items: stretch;
    overflow: hidden;
    min-height: 120px;
  }

  .card-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--spacing-1);
    padding: var(--spacing-4);
    min-width: 0;
  }

  .name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    text-transform: uppercase;
    line-height: 1.6;
  }

  .region {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-secondary);
  }
</style>
