<script lang="ts">
  import InfoBox from '$lib/components/InfoBox.svelte';
  import DataList from '$lib/components/DataList.svelte';

  let { data } = $props();

  let title = $derived(
    data.versions.length > 0
      ? data.versions.map(v => v.name.toUpperCase().replace(/-/g, ' ')).join(' / ')
      : data.versionGroup.name.toUpperCase().replace(/-/g, ' ')
  );

  function formatGenName(name: string): string {
    return name.replace('generation-', 'GENERATION ').toUpperCase();
  }

  let aboutItems = $derived([
    { label: 'SLUG', value: data.versionGroup.slug },
    ...(data.generation ? [{ label: 'GENERATION', value: formatGenName(data.generation.name) }] : []),
    { label: 'VERSIONS', value: String(data.versions.length) },
  ]);
</script>

<svelte:head>
  <title>{data.versionGroup.name} | Games | Karta Yaxu</title>
</svelte:head>

<div class="games-detail">
  <header class="detail-header">
    <h1>{title}</h1>
    {#if data.generation}
      <span class="gen-label">{formatGenName(data.generation.name)}</span>
    {/if}
  </header>

  <InfoBox title="VERSIONS">
    {#if data.versions.length === 0}
      <p class="empty">No versions found.</p>
    {:else}
      <div class="versions-grid">
        {#each data.versions as v (v.id)}
          <div class="version-card">
            <span class="version-name">{v.name.toUpperCase().replace(/-/g, ' ')}</span>
            <span class="version-slug">{v.slug}</span>
          </div>
        {/each}
      </div>
    {/if}
  </InfoBox>

  <InfoBox title="ABOUT">
    <DataList items={aboutItems} />
  </InfoBox>
</div>

<style>
  .games-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-6);
    box-shadow: var(--shadow-card);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .gen-label {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
  }

  .versions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--spacing-4);
  }

  .version-card {
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
    min-height: 120px;
    justify-content: center;
  }

  .version-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    line-height: 1.6;
  }

  .version-slug {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .empty {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
    font-size: 13px;
    margin: 0;
  }

  @media (max-width: 768px) {
    h1 { font-size: 18px; }
    .detail-header { padding: var(--spacing-4); }
    .versions-grid { grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); }
  }
</style>
