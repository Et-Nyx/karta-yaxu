<script lang="ts">
  // Wiki Landing Page
</script>

<svelte:head>
  <title>Wiki | Karta Yaxu</title>
</svelte:head>

<div class="wiki-home">
  <div class="header-section">
    <h2>WIKI / DATABASE</h2>
    <p>Explore the Karta Yaxu knowledge base. Data is strictly read-only on this section.</p>
  </div>

  <div class="menu-grid">
    <a href="/wiki/pokemon" class="menu-card">
      <div class="icon">🐾</div>
      <h3>Pokédex</h3>
      <p>Browse all Pokémon, their stats, types, and evolutions.</p>
    </a>

    <a href="/wiki/moves" class="menu-card">
      <div class="icon">⚔️</div>
      <h3>Moves</h3>
      <p>Explore attacks, status moves, and their detailed mechanics.</p>
    </a>

    <a href="/wiki/abilities" class="menu-card">
      <div class="icon">✨</div>
      <h3>Abilities</h3>
      <p>List of all passive abilities and their effects in battle.</p>
    </a>

    <a href="/wiki/types" class="menu-card">
      <div class="icon">🔥</div>
      <h3>Types</h3>
      <p>Type match-ups, weaknesses, and resistances.</p>
    </a>
  </div>
</div>

<style>
  .wiki-home {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .header-section h2 {
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
  }

  .header-section p {
    color: var(--theme-text-secondary);
    font-size: 14px;
    margin: 0;
  }

  .menu-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: var(--spacing-4);
  }

  .menu-card {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-5);
    text-decoration: none;
    color: inherit;
    transition: all 0.1s ease;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .menu-card:hover {
    transform: translate(-2px, -2px);
    box-shadow: var(--shadow-card-hover);
    background-color: var(--theme-bg-hover);
  }

  .icon {
    font-size: 32px;
    margin-bottom: var(--spacing-3);
  }

  .menu-card h3 {
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
  }

  .menu-card p {
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin: 0;
  }
</style>
