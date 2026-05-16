<script lang="ts">
  let { data } = $props();

  function formatGenName(name: string): string {
    return name.replace('generation-', 'GEN ').toUpperCase();
  }

  function formatVersionNames(versions: { name: string }[]): string {
    return versions.map(v => v.name.toUpperCase().replace(/-/g, ' ')).join(' / ');
  }
</script>

<svelte:head>
  <title>Games | Karta Yaxu</title>
</svelte:head>

<div class="games-page">
  <div class="page-header">
    <h2>GAMES</h2>
    <p class="subtitle">All generations and version groups</p>
  </div>

  {#if data.generations.length === 0}
    <div class="empty-state">
      <p>No game data imported yet.</p>
      <p class="hint">Use the <a href="/import">Import Panel</a> to import game metadata.</p>
    </div>
  {:else}
    {#each data.generations as gen}
      <section class="gen-section">
        <h3 class="gen-title">{formatGenName(gen.name)}</h3>
        <div class="vg-grid">
          {#each gen.versionGroups as vg (vg.id)}
            <a href="/wiki/games/{vg.slug}" class="vg-card card-interactive">
              <div class="vg-name">{vg.name.toUpperCase().replace(/-/g, ' ')}</div>
              {#if vg.versions.length > 0}
                <div class="version-list">
                  {#each vg.versions as v}
                    <span class="version-pill">{v.name.toUpperCase()}</span>
                  {/each}
                </div>
              {/if}
            </a>
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</div>

<style>
  .games-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .subtitle {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0;
  }

  .gen-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .gen-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .vg-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-3);
  }

  .vg-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
    padding: var(--spacing-4);
    min-width: 160px;
  }

  .vg-name {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    letter-spacing: 0.5px;
  }

  .version-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-1);
  }

  .version-pill {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    padding: 1px 6px;
  }

  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-8);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint {
    font-size: 12px;
    color: var(--theme-text-muted);
  }

  .empty-state a {
    color: var(--theme-text-accent);
    text-decoration: none;
  }
</style>
