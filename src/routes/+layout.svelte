<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import '../app.css';

  let { children } = $props();

  // Se estamos na rota /import ou /editor, forçamos o tema admin (verde gameboy)
  let isAdminRoute = $derived($page.url.pathname.startsWith('/import') || $page.url.pathname.startsWith('/editor'));

  let selectedTheme = $state('auto');

  // Atualiza o DOM e o localStorage quando o tema mudar
  function applyTheme(theme: string) {
    if (typeof document === 'undefined') return;
    
    if (isAdminRoute) {
      document.documentElement.setAttribute('data-theme', 'admin');
    } else {
      if (theme === 'auto') {
        document.documentElement.removeAttribute('data-theme');
      } else {
        document.documentElement.setAttribute('data-theme', theme);
      }
    }
    
    if (theme !== 'auto') {
      localStorage.setItem('theme', theme);
    } else {
      localStorage.removeItem('theme');
    }
  }

  // Monitora mudanças manuais
  $effect(() => {
    applyTheme(selectedTheme);
  });

  // Carrega a preferência inicial
  onMount(() => {
    const saved = localStorage.getItem('theme');
    if (saved) {
      selectedTheme = saved;
    }
  });

  // Gera breadcrumbs com links baseados na URL
  let breadcrumbs = $derived(() => {
    const paths = $page.url.pathname.split('/').filter(Boolean);
    
    const crumbs: { label: string; href: string }[] = [{ label: 'HOME', href: '/' }];
    let currentPath = '';
    
    for (const segment of paths) {
      currentPath += `/${segment}`;
      // Beautify labels: replace hyphens with spaces, uppercase
      const label = segment.toUpperCase().replace(/-/g, ' ');
      crumbs.push({ label, href: currentPath });
    }
    return crumbs;
  });

  let parentHref = $derived(() => {
    const crumbs = breadcrumbs();
    return crumbs.length >= 2 ? crumbs[crumbs.length - 2].href : '/';
  });
</script>

<div class="app-container" data-theme={isAdminRoute ? 'admin' : null}>
  <div class="gameboy-frame">
    <!-- Header -->
    <header class="header">
      <div class="logo">
        <a href="/">KARTA YAXU</a>
        <span class="edition">GAMEBOY EDITION</span>
      </div>
      <div class="nav-tabs">
        <select bind:value={selectedTheme} class="theme-select">
          <option value="auto">Theme: Auto</option>
          <option value="light">Theme: Light</option>
          <option value="dark">Theme: Dark</option>
          <option value="admin">Theme: Admin (Verde)</option>
        </select>
        <a href="/wiki" class="tab" class:active={$page.url.pathname.startsWith('/wiki')}>WIKI</a>
        <a href="/editor" class="tab" class:active={$page.url.pathname.startsWith('/editor')}>EDITOR</a>
        <a href="/import" class="tab" class:active={$page.url.pathname.startsWith('/import')}>IMPORT</a>
        <a href="/battle" class="tab" class:active={$page.url.pathname.startsWith('/battle')}>BATTLE</a>
        <a href="/trainer/login" class="tab login">LOGIN</a>
      </div>
    </header>

    <!-- Main Layout: Sidebar + Content -->
    <div class="layout">
      <!-- Sidebar -->
      <aside class="sidebar">
        <nav class="side-nav">
          <div class="nav-section">
            <h3>WIKI</h3>
            <a href="/wiki/pokemon">Pokémon</a>
            <a href="/wiki/moves">Moves</a>
            <a href="/wiki/abilities">Abilities</a>
            <a href="/wiki/types">Types</a>
            <a href="/wiki/items">Items</a>
            <a href="/wiki/locations">Locations</a>
          </div>

          {#if isAdminRoute}
            <div class="nav-section">
              <h3>ADMIN</h3>
              <a href="/editor">Database Editor</a>
              <a href="/import">PokeAPI Import</a>
            </div>
          {/if}
        </nav>
        
        <div class="version-select">
          <label for="vg-select">VERSION</label>
          <select id="vg-select">
            <option>FireRed / LeafGreen</option>
            <option>Ruby / Sapphire</option>
            <option>Emerald</option>
          </select>
        </div>
      </aside>

      <!-- Content Area -->
      <main class="content-wrapper">
        <!-- Breadcrumbs -->
        <div class="breadcrumbs">
          <a href={parentHref()} class="back-link">← BACK</a>
          <span class="breadcrumb-divider">|</span>
          {#each breadcrumbs() as crumb, i}
            {#if i < breadcrumbs().length - 1}
              <a href={crumb.href} class="crumb">{crumb.label}</a>
              <span class="separator">&gt;</span>
            {:else}
              <span class="crumb active">{crumb.label}</span>
            {/if}
          {/each}
        </div>

        <div class="content">
          {@render children()}
        </div>
      </main>
    </div>
  </div>
</div>

<style>
  .app-container {
    min-height: 100vh;
    background-color: #000;
    display: flex;
    justify-content: center;
    padding: 2vw; /* Responsividade invés de pixels fixos */
  }

  .gameboy-frame {
    width: 100%;
    max-width: 1400px;
    background-color: var(--theme-bg-primary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: inset 0 0 20px rgba(0,0,0,0.2), 6px 6px 0px rgba(255,255,255,0.1);
    display: flex;
    flex-direction: column;
    min-height: 90vh;
  }

  /* Header */
  .header {
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    border-bottom: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .logo a {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-accent);
    text-shadow: 2px 2px 0px #000;
  }

  .logo .edition {
    display: block;
    font-family: var(--font-mono);
    font-size: 10px;
    margin-top: 4px;
    opacity: 0.8;
  }

  .nav-tabs {
    display: flex;
    gap: var(--spacing-3);
  }

  .tab {
    font-family: var(--font-display);
    font-size: 12px;
    padding: var(--spacing-2) var(--spacing-3);
    color: var(--theme-text-inverse);
    border: 2px solid transparent;
  }

  .tab:hover {
    color: var(--theme-text-accent);
  }

  .tab.active {
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .tab.login {
    border: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .theme-select {
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px var(--spacing-2);
    margin-right: var(--spacing-4);
    cursor: pointer;
  }

  .theme-select:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  /* Layout */
  .layout {
    display: flex;
    flex: 1;
  }

  /* Sidebar */
  .sidebar {
    width: 20%;
    min-width: 200px;
    max-width: 260px;
    border-right: 4px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    display: flex;
    flex-direction: column;
    padding: var(--spacing-5) var(--spacing-4);
  }

  .nav-section {
    margin-bottom: var(--spacing-6);
  }

  .nav-section h3 {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-muted);
    margin-bottom: var(--spacing-3);
  }

  .nav-section a {
    display: block;
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    padding: var(--spacing-2);
    margin-bottom: var(--spacing-1);
    border: 2px solid transparent;
  }

  .nav-section a:hover {
    background-color: var(--theme-bg-hover);
    border-color: var(--theme-border-primary);
  }

  .version-select {
    margin-top: auto;
  }

  .version-select label {
    display: block;
    font-family: var(--font-display);
    font-size: 10px;
    margin-bottom: var(--spacing-2);
  }

  .version-select select {
    width: 100%;
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }

  /* Content */
  .content-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: var(--spacing-6);
    background-color: var(--theme-bg-primary);
  }

  .breadcrumbs {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
    margin-bottom: var(--spacing-6);
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    flex-wrap: wrap;
  }

  .back-link {
    color: var(--theme-text-accent);
    text-decoration: none;
    font-family: var(--font-display);
    font-size: 10px;
  }

  .back-link:hover {
    text-decoration: underline;
  }

  .breadcrumb-divider {
    opacity: 0.4;
  }

  .crumb {
    color: var(--theme-text-secondary);
    text-decoration: none;
  }

  .crumb:not(.active):hover {
    color: var(--theme-text-primary);
    text-decoration: underline;
  }

  .crumb.active {
    color: var(--theme-text-primary);
    font-weight: bold;
    pointer-events: none;
  }

  .separator {
    margin: 0 var(--spacing-2);
    opacity: 0.5;
  }

  .content {
    flex: 1;
    max-width: 1000px;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .layout {
      flex-direction: column;
    }
    .sidebar {
      width: 100%;
      max-width: none;
      border-right: none;
      border-bottom: 4px solid var(--theme-border-primary);
    }
    .header {
      flex-direction: column;
      gap: var(--spacing-4);
    }
    .nav-tabs {
      flex-wrap: wrap;
      justify-content: center;
    }
    .app-container {
      padding: 0;
    }
  }
</style>
