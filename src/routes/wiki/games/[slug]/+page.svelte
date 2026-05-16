<script lang="ts">
  let { data } = $props();

  let title = $derived(
    data.versions.length > 0
      ? data.versions.map(v => v.name.toUpperCase().replace(/-/g, ' ')).join(' / ')
      : data.versionGroup.name.toUpperCase().replace(/-/g, ' ')
  );

  function formatGenName(name: string): string {
    return name.replace('generation-', 'GENERATION ').toUpperCase();
  }
</script>

<svelte:head>
  <title>{data.versionGroup.name} | Games | Karta Yaxu</title>
</svelte:head>

<div class="games-detail">
  <div class="detail-header">
    <h2>{title}</h2>
    {#if data.generation}
      <a href="/wiki/games" class="gen-badge">{formatGenName(data.generation.name)}</a>
    {/if}
  </div>

  <div class="detail-sections">
    <section class="detail-section">
      <h3 class="section-title">VERSIONS</h3>
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
    </section>

    <section class="detail-section">
      <h3 class="section-title">ABOUT</h3>
      <div class="info-table">
        <div class="info-row">
          <span class="info-label">SLUG</span>
          <span class="info-value">{data.versionGroup.slug}</span>
        </div>
        {#if data.generation}
          <div class="info-row">
            <span class="info-label">GENERATION</span>
            <span class="info-value">{formatGenName(data.generation.name)}</span>
          </div>
        {/if}
        <div class="info-row">
          <span class="info-label">VERSIONS</span>
          <span class="info-value">{data.versions.length}</span>
        </div>
      </div>
    </section>
  </div>
</div>

<style>
  .games-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .detail-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .gen-badge {
    display: inline-block;
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    padding: 2px 10px;
    text-decoration: none;
    width: fit-content;
  }

  .gen-badge:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .detail-sections {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .versions-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-3);
  }

  .version-card {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
    min-width: 120px;
  }

  .version-name {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .version-slug {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .info-table {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .info-row {
    display: flex;
    gap: var(--spacing-4);
    padding: var(--spacing-2) 0;
    border-bottom: 1px solid var(--theme-border-primary);
  }

  .info-label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    min-width: 100px;
  }

  .info-value {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-primary);
  }

  .empty {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
    font-size: 13px;
  }
</style>
