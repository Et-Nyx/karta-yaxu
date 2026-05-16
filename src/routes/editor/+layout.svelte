<script lang="ts">
  import { page } from '$app/stores';

  let { children } = $props();

  const ready = [
    { href: '/editor/types',     label: 'TYPES' },
    { href: '/editor/natures',   label: 'NATURES' },
    { href: '/editor/abilities', label: 'ABILITIES' },
    { href: '/editor/items',     label: 'ITEMS' },
    { href: '/editor/pokedex',   label: 'POKÉDEX' },
    { href: '/editor/moves',     label: 'MOVES' },
    { href: '/editor/pokemon',   label: 'POKÉMON' },
    { href: '/editor/species',   label: 'SPECIES' },
  ];

  const comingSoon = [
    { label: 'LEARNSET' },
    { label: 'EVOLUTION' },
  ];
</script>

<div class="editor-layout">
  <aside class="editor-sidebar">
    <h3 class="section-title">DATABASE EDITOR</h3>
    <nav class="editor-nav">
      <a href="/editor" class="nav-link" class:active={$page.url.pathname === '/editor'}>
        DASHBOARD
      </a>
      {#each ready as entry}
        <a
          href={entry.href}
          class="nav-link"
          class:active={$page.url.pathname.startsWith(entry.href)}
        >
          {entry.label}
        </a>
      {/each}
    </nav>

    <h3 class="section-title muted">COMING SOON</h3>
    <nav class="editor-nav">
      {#each comingSoon as entry}
        <span class="nav-link disabled">{entry.label}</span>
      {/each}
    </nav>
  </aside>

  <div class="editor-content">
    {@render children()}
  </div>
</div>

<style>
  .editor-layout {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: var(--spacing-6);
    align-items: start;
  }

  .editor-sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
    border: 3px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    padding: var(--spacing-4);
    position: sticky;
    top: var(--spacing-4);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .section-title.muted {
    color: var(--theme-text-muted);
    margin-top: var(--spacing-3);
  }

  .editor-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-link {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2);
    color: var(--theme-text-primary);
    border: 2px solid transparent;
    text-decoration: none;
  }

  .nav-link:hover {
    background-color: var(--theme-bg-hover);
    border-color: var(--theme-border-primary);
  }

  .nav-link.active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .nav-link.disabled {
    color: var(--theme-text-muted);
    cursor: not-allowed;
    opacity: 0.5;
  }

  .editor-content {
    min-width: 0;
  }

  @media (max-width: 768px) {
    .editor-layout {
      grid-template-columns: 1fr;
    }

    .editor-sidebar {
      position: static;
    }
  }
</style>
