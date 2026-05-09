<svelte:head>
  <title>Karta Yaxu — Pokémon ROM Hack Wiki & Simulator</title>
  <meta name="description" content="Wiki, Editor & Battle Simulator for Pokémon Gen 3 ROM Hack" />
</svelte:head>

<div class="home-content">
  <div class="welcome-box">
    <h1>BEM-VINDO A KARTA YAXU</h1>
    <p>O simulador e wiki open-source para a sua ROM hack da Geração 3.</p>
    
    <div class="quick-links">
      <a href="/wiki/pokemon" class="btn">Pokédex (Wiki)</a>
      <a href="/import" class="btn">Importar Dados (Admin)</a>
    </div>
  </div>
</div>

<style>
  .home-content {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    padding: var(--spacing-6) 0;
  }

  .welcome-box {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-8);
    text-align: center;
    max-width: 600px;
    width: 100%;
  }

  h1 {
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-4);
  }

  p {
    color: var(--theme-text-secondary);
    margin-bottom: var(--spacing-6);
  }

  .quick-links {
    display: flex;
    gap: var(--spacing-4);
    justify-content: center;
  }

  .btn {
    display: inline-block;
    padding: var(--spacing-3) var(--spacing-4);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
  }

  .btn:hover {
    background-color: var(--theme-bg-hover);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0px var(--theme-border-primary);
  }
</style>
