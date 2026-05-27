<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import '../app.css';
  import ImportActivity from '$lib/components/ImportActivity.svelte';
  import { handleImportEvent, stagingActivity } from '$lib/stores/importActivity';
  import { pageSource } from '$lib/stores/pageSource';
  import NavTab from '$lib/components/NavTab.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import RetroInput from '$lib/components/RetroInput.svelte';
  import RetroButton from '$lib/components/RetroButton.svelte';

  let { children, data } = $props();

  let isAdminRoute = $derived($page.url.pathname.startsWith('/import') || $page.url.pathname.startsWith('/editor'));
  let isEditorRoute = $derived($page.url.pathname.startsWith('/editor'));

  const editorSections = new Set(['pokemon', 'types', 'moves', 'abilities', 'items', 'natures', 'locations', 'pokedex']);
  const wikiSections   = new Set(['pokemon', 'types', 'moves', 'abilities', 'items', 'natures', 'locations', 'pokedex']);

  let wikiUrl = $derived.by(() => {
    const path = $page.url.pathname;
    if (!path.startsWith('/editor/')) return null;
    const rest    = path.slice('/editor/'.length);
    const parts   = rest.split('/');
    const section = parts[0];
    const slug    = parts[1];
    
    if (!slug) {
      if (wikiSections.has(section)) return `/wiki/${section}`;
      if (section === 'species') return `/wiki/pokemon`;
      if (section === 'trainers') return `/trainer`;
      return null;
    }
    
    if (wikiSections.has(section)) return `/wiki/${rest}`;
    if (section === 'species')  return `/wiki/pokemon/${slug}`;
    if (section === 'trainers') return `/trainer/${slug}`;
    return null;
  });

  let editorUrl = $derived.by(() => {
    if (data.user?.role !== 'editor') return null;
    const path = $page.url.pathname;

    if (path === '/trainer') return '/editor/trainers';
    if (path.startsWith('/trainer/')) {
      const rest = path.slice('/trainer/'.length);
      if (rest && !['login', 'register'].includes(rest)) return `/editor/trainers/${rest}`;
      return null;
    }

    if (path.startsWith('/wiki/')) {
      const rest = path.slice('/wiki/'.length);
      const section = rest.split('/')[0];
      if (editorSections.has(section)) return `/editor/${rest}`;
    }

    return null;
  });

  let selectedTheme = $state('auto');
  let mobileMenuOpen = $state(false);

  const themeOptions = [
    { value: 'auto', label: 'Theme: Auto' },
    { value: 'light', label: 'Theme: Light' },
    { value: 'dark', label: 'Theme: Dark' },
    { value: 'admin', label: 'Theme: Admin' }
  ];

  // Close mobile menu on navigation
  $effect(() => {
    $page.url.pathname;
    mobileMenuOpen = false;
  });

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

  $effect(() => {
    applyTheme(selectedTheme);
  });

  $effect(() => {
    if (data.activeStaged) {
      stagingActivity.set({
        entityType: data.activeStaged.entityType,
        fetched: data.activeStaged.fetched,
        failed: data.activeStaged.failed,
        total: data.activeStaged.total,
        status: data.activeStaged.status as 'fetching' | 'fetched' | 'failed' | 'idle',
      });
    }
  });

  onMount(() => {
    const saved = localStorage.getItem('theme');
    if (saved) selectedTheme = saved;

    const lastId = localStorage.getItem('lastSeenImportId') || '0';
    const es = new EventSource(`/api/import/sse?lastId=${lastId}`);

    es.onmessage = (e) => {
      try {
        const ev = JSON.parse(e.data);
        if (ev.id) {
          localStorage.setItem('lastSeenImportId', String(ev.id));
        }

        if (ev.entityType === 'staging') {
          const commentObj = JSON.parse(ev.comment || '{}');
          stagingActivity.set({
            entityType: commentObj.entityType || 'pokemon',
            fetched: commentObj.fetched || 0,
            failed: commentObj.error ? 1 : 0,
            total: commentObj.total || 1,
            status: commentObj.total === commentObj.fetched ? 'fetched' : 'fetching',
            activeName: null,
            activeId: null,
          });
        } else if (ev.entityType === 'migrating') {
          const commentObj = JSON.parse(ev.comment || '{}');
          stagingActivity.set({
            entityType: 'pokemon',
            fetched: commentObj.index || 0,
            failed: 0,
            total: commentObj.total || 1,
            status: 'migrating',
            activeName: commentObj.name || null,
            activeId: commentObj.id || null,
          });
        } else {
          handleImportEvent(ev);
        }
      } catch {}
    };

    // Global capturing event listener for form submissions to intercept BEFORE SvelteKit use:enhance
    const submitListener = (e: SubmitEvent) => {
      handleGlobalSubmit(e);
    };
    window.addEventListener('submit', submitListener, { capture: true });

    return () => {
      es.close();
      window.removeEventListener('submit', submitListener, { capture: true });
    };
  });

  let breadcrumbs = $derived(() => {
    const paths = $page.url.pathname.split('/').filter(Boolean);
    const crumbs: { label: string; href: string }[] = [{ label: 'HOME', href: '/' }];
    let currentPath = '';
    for (const segment of paths) {
      currentPath += `/${segment}`;
      const label = segment.toUpperCase().replace(/-/g, ' ');
      crumbs.push({ label, href: currentPath });
    }
    return crumbs;
  });

  let parentHref = $derived(() => {
    const crumbs = breadcrumbs();
    return crumbs.length >= 2 ? crumbs[crumbs.length - 2].href : '/';
  });

  // Global interceptor for Edit modifications
  let showEditPrompt = $state(false);
  let editCommentText = $state('');
  let pendingForm = $state<HTMLFormElement | null>(null);

  function handleGlobalSubmit(e: SubmitEvent) {
    if (typeof document === 'undefined') return;
    const form = e.target as HTMLFormElement;
    if (!form || form.method.toUpperCase() !== 'POST') return;
    
    // Skip if not an editor sub-route, or if it is the main dashboard, or if it has auditComment already
    const path = $page.url.pathname;
    if (!path.startsWith('/editor') || path === '/editor') return;
    if (form.querySelector('input[name="auditComment"]')) return;

    // Intercept!
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();
    pendingForm = form;
    editCommentText = '';
    showEditPrompt = true;
  }

  function confirmPendingSubmit() {
    if (!pendingForm) return;
    const input = document.createElement('input');
    input.type = 'hidden';
    input.name = 'auditComment';
    input.value = editCommentText.trim();
    pendingForm.appendChild(input);
    
    const formToSubmit = pendingForm;
    pendingForm = null;
    showEditPrompt = false;
    formToSubmit.requestSubmit();
  }

  function cancelPendingSubmit() {
    pendingForm = null;
    showEditPrompt = false;
  }
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
        <RetroSelect bind:value={selectedTheme} options={themeOptions} variant="header" class="theme-select" />
        <NavTab href="/wiki" active={$page.url.pathname.startsWith('/wiki')}>WIKI</NavTab>
        <NavTab href="/editor" active={$page.url.pathname.startsWith('/editor')}>EDITOR</NavTab>
        <NavTab href="/battle" active={$page.url.pathname.startsWith('/battle')}>BATTLE</NavTab>
        {#if data.user}
          <NavTab href="/trainer/{data.user.username}" active={$page.url.pathname.startsWith('/trainer/' + data.user.username)}>{data.user.username.toUpperCase()}</NavTab>
          <form action="/trainer/logout" method="post" style="display:contents">
            <NavTab>LOGOUT</NavTab>
          </form>
        {:else}
          <NavTab href="/trainer/login" variant="login">LOGIN</NavTab>
        {/if}
      </div>
      <button class="hamburger" onclick={() => mobileMenuOpen = !mobileMenuOpen} aria-label="Menu">
        {mobileMenuOpen ? '✕' : '☰'}
      </button>
    </header>

    <!-- Main Layout: Sidebar + Content -->
    <div class="layout">
      <!-- Mobile backdrop -->
      {#if mobileMenuOpen}
        <div class="backdrop" onclick={() => mobileMenuOpen = false} role="presentation"></div>
      {/if}

      <!-- Sidebar -->
      <aside class="sidebar" class:open={mobileMenuOpen}>
        <!-- Mobile-only: nav tabs + theme selector inside drawer -->
        <div class="mobile-nav-tabs">
          <RetroSelect bind:value={selectedTheme} options={themeOptions} variant="header" class="theme-select-mobile" />
          <a href="/wiki"    class="mobile-tab" class:active={$page.url.pathname.startsWith('/wiki')}>WIKI</a>
          <a href="/editor"  class="mobile-tab" class:active={$page.url.pathname.startsWith('/editor')}>EDITOR</a>
          <a href="/battle"  class="mobile-tab" class:active={$page.url.pathname.startsWith('/battle')}>BATTLE</a>
          {#if data.user}
            <a href="/trainer/{data.user.username}" class="mobile-tab">{data.user.username.toUpperCase()}</a>
            <form action="/trainer/logout" method="post">
              <button type="submit" class="mobile-tab mobile-tab-btn">LOGOUT</button>
            </form>
          {:else}
            <a href="/trainer/login" class="mobile-tab">LOGIN</a>
          {/if}
        </div>

        <nav class="side-nav">
          {#if !isAdminRoute}
          <div class="nav-section">
            <h3>WIKI</h3>
            <a href="/wiki"            class:active={$page.url.pathname === '/wiki'}>Home</a>
            <a href="/wiki/pokemon"    class:active={$page.url.pathname.startsWith('/wiki/pokemon')}>Pokémon</a>
            <a href="/wiki/types"      class:active={$page.url.pathname.startsWith('/wiki/types')}>Types</a>
            <a href="/wiki/moves"      class:active={$page.url.pathname.startsWith('/wiki/moves')}>Moves</a>
            <a href="/wiki/abilities"  class:active={$page.url.pathname.startsWith('/wiki/abilities')}>Abilities</a>
            <a href="/wiki/items"      class:active={$page.url.pathname.startsWith('/wiki/items')}>Items</a>
            <a href="/wiki/natures"    class:active={$page.url.pathname.startsWith('/wiki/natures')}>Natures</a>
            <a href="/wiki/locations"  class:active={$page.url.pathname.startsWith('/wiki/locations')}>Locations</a>
            <a href="/wiki/pokedex"    class:active={$page.url.pathname.startsWith('/wiki/pokedex')}>Pokédex</a>
            <a href="/trainer"         class:active={$page.url.pathname.startsWith('/trainer')}>Trainers</a>
            <a href="/wiki/games"      class:active={$page.url.pathname.startsWith('/wiki/games')}>Games</a>
          </div>
          {/if}

          {#if data.user?.role === 'editor'}
            {#if isAdminRoute}
              <div class="nav-section">
                <h3>EDITOR</h3>
                <a href="/editor"           class:active={$page.url.pathname === '/editor'}>Dashboard</a>
                <a href="/editor/pokemon"   class:active={$page.url.pathname.startsWith('/editor/pokemon')}>Pokémon</a>
                <a href="/editor/types"     class:active={$page.url.pathname.startsWith('/editor/types')}>Types</a>
                <a href="/editor/moves"     class:active={$page.url.pathname.startsWith('/editor/moves')}>Moves</a>
                <a href="/editor/abilities" class:active={$page.url.pathname.startsWith('/editor/abilities')}>Abilities</a>
                <a href="/editor/items"     class:active={$page.url.pathname.startsWith('/editor/items')}>Items</a>
                <a href="/editor/natures"    class:active={$page.url.pathname.startsWith('/editor/natures')}>Natures</a>
                <a href="/editor/locations" class:active={$page.url.pathname.startsWith('/editor/locations')}>Locations</a>
                <a href="/editor/pokedex"   class:active={$page.url.pathname.startsWith('/editor/pokedex')}>Pokédex</a>
                <a href="/editor/trainers"  class:active={$page.url.pathname.startsWith('/editor/trainers')}>Trainers</a>
                <a href="/editor/species"   class:active={$page.url.pathname.startsWith('/editor/species')}>Species</a>
              </div>
            {/if}
          {/if}
        </nav>

        <ImportActivity />

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
          {#if editorUrl || wikiUrl}
            <div class="breadcrumb-actions">
              {#if editorUrl}
                <a href={editorUrl} class="edit-this-link">EDIT THIS →</a>
              {/if}
              {#if wikiUrl}
                {#if $pageSource}
                  <span class="src-badge src-{$pageSource}">{$pageSource}</span>
                {/if}
                <a href={wikiUrl} class="edit-this-link">VIEW IN WIKI →</a>
              {/if}
            </div>
          {/if}
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
    height: 100vh;
    overflow: hidden;
    background-color: #000;
    display: flex;
    justify-content: center;
    padding: 2vw;
  }

  .gameboy-frame {
    width: 100%;
    max-width: 1400px;
    background-color: var(--theme-bg-primary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: inset 0 0 20px rgba(0,0,0,0.2), 6px 6px 0px rgba(255,255,255,0.1);
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  /* ── Header ─────────────────────────────────────────────────────────────── */

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
    align-items: center;
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

  .hamburger {
    display: none;
    background: none;
    border: 2px solid var(--theme-text-inverse);
    color: var(--theme-text-inverse);
    font-size: 18px;
    padding: 4px 10px;
    cursor: pointer;
    font-family: var(--font-display);
    line-height: 1;
  }

  /* ── Layout ──────────────────────────────────────────────────────────────── */

  .layout {
    display: flex;
    flex: 1;
    position: relative;
    min-height: 0;
    overflow: hidden;
  }

  /* ── Backdrop (mobile) ───────────────────────────────────────────────────── */

  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 99;
  }

  /* ── Sidebar ─────────────────────────────────────────────────────────────── */

  .sidebar {
    width: 20%;
    min-width: 200px;
    max-width: 260px;
    border-right: 4px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    display: flex;
    flex-direction: column;
    padding: var(--spacing-5) var(--spacing-4);
    overflow-y: auto;
  }

  .mobile-nav-tabs {
    display: none;
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

  .nav-section a.active {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .edit-this-link {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-accent);
    text-decoration: none;
    flex-shrink: 0;
  }

  .edit-this-link:hover {
    text-decoration: underline;
  }

  .breadcrumb-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    flex-shrink: 0;
  }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
  }
  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }
  .src-unsaved { color: var(--color-text-error); border-color: var(--color-text-error); text-transform: uppercase; }

  .nav-link-disabled {
    display: block;
    font-family: var(--font-display);
    font-size: 12px;
    padding: var(--spacing-2);
    margin-bottom: var(--spacing-1);
    color: var(--theme-text-muted);
    opacity: 0.45;
    cursor: not-allowed;
  }


  /* ── Content ─────────────────────────────────────────────────────────────── */

  .content-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: var(--spacing-6);
    background-color: var(--theme-bg-primary);
    min-width: 0;
    overflow-y: auto;
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

  /* ── Mobile ──────────────────────────────────────────────────────────────── */

  @media (max-width: 992px) {
    .app-container {
      padding: 0;
      height: auto;
      min-height: 100vh;
      overflow-x: hidden;
      overflow-y: visible;
    }

    .gameboy-frame {
      height: auto;
      min-height: 100vh;
      overflow: visible;
    }

    .layout {
      overflow: visible;
    }

    .content-wrapper {
      overflow-y: visible;
    }

    .header {
      height: 60px;
      padding: 0 var(--spacing-4);
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      z-index: 101;
      box-sizing: border-box;
    }

    .layout {
      padding-top: 60px;
    }

    .nav-tabs {
      display: none;
    }

    .hamburger {
      display: block;
    }

    /* Sidebar becomes a fixed slide-in drawer */
    .sidebar {
      position: fixed;
      left: 0;
      top: 60px;
      height: calc(100vh - 60px);
      width: 280px;
      max-width: 85vw;
      z-index: 100;
      transform: translateX(-100%);
      transition: transform 0.2s ease;
      overflow-y: auto;
      padding-top: var(--spacing-4);
      border-right: 4px solid var(--theme-border-primary);
    }

    .sidebar.open {
      transform: translateX(0);
    }

    /* Mobile nav tabs inside the drawer */
    .mobile-nav-tabs {
      display: flex;
      flex-direction: column;
      gap: var(--spacing-1);
      padding-bottom: var(--spacing-4);
      border-bottom: 2px solid var(--theme-border-primary);
      margin-bottom: var(--spacing-4);
    }

    .theme-select-mobile {
      background-color: var(--theme-bg-primary);
      color: var(--theme-text-primary);
      border: 2px solid var(--theme-border-primary);
      font-family: var(--font-display);
      font-size: 8px;
      padding: 4px var(--spacing-2);
      margin-bottom: var(--spacing-2);
      width: 100%;
      cursor: pointer;
    }

    .mobile-tab {
      display: block;
      font-family: var(--font-display);
      font-size: 11px;
      color: var(--theme-text-primary);
      padding: var(--spacing-2) var(--spacing-3);
      border: 2px solid transparent;
      text-decoration: none;
    }

    .mobile-tab:hover,
    .mobile-tab.active {
      color: var(--theme-text-accent);
      border-color: var(--theme-text-accent);
    }

    .mobile-tab-btn {
      width: 100%;
      text-align: left;
      background: none;
      cursor: pointer;
    }

    /* Content fills full width */
    .layout {
      flex-direction: column;
      width: 100%;
      max-width: 100%;
    }

    .content-wrapper {
      padding: var(--spacing-4);
      width: 100%;
      max-width: 100%;
      box-sizing: border-box;
    }

    .content {
      width: 100%;
      max-width: 100%;
      box-sizing: border-box;
    }
  }

  /* Retro Edit Confirmation Modal styling */
  .retro-modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background-color: rgba(0, 0, 0, 0.7);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 10000;
  }

  .retro-modal {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: 8px 8px 0 var(--theme-border-primary);
    width: 320px;
    max-width: 90%;
    display: flex;
    flex-direction: column;
  }

  .retro-modal-header {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 4px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .retro-modal-header h3 {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-inverse);
    margin: 0;
  }

  .retro-modal-header .btn-close {
    background: none;
    border: none;
    color: var(--theme-text-inverse);
    font-family: var(--font-mono);
    font-size: 16px;
    font-weight: bold;
    cursor: pointer;
    padding: 0 var(--spacing-1);
  }

  .retro-modal-body {
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .prompt-text {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-primary);
    margin: 0;
    line-height: 1.4;
  }

  :global(.modal-retro-input) {
    width: 100%;
    box-sizing: border-box;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-3);
  }
</style>

{#if showEditPrompt}
  <div class="retro-modal-overlay">
    <div class="retro-modal">
      <div class="retro-modal-header">
        <h3>CONFIRM CHANGES</h3>
        <button type="button" class="btn-close" onclick={cancelPendingSubmit}>×</button>
      </div>
      <div class="retro-modal-body">
        <p class="prompt-text">ENTER REASON FOR THIS MODIFICATION (COMMIT MESSAGE):</p>
        <div class="comment-field">
          <RetroInput
            type="text"
            placeholder="e.g. Corrected stat value to match Gen 9"
            bind:value={editCommentText}
            autofocus
            class="modal-retro-input"
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                confirmPendingSubmit();
              }
            }}
          />
        </div>
        <div class="modal-actions">
          <RetroButton onclick={confirmPendingSubmit} variant="primary">CONFIRM</RetroButton>
          <RetroButton onclick={cancelPendingSubmit} variant="secondary">CANCEL</RetroButton>
        </div>
      </div>
    </div>
  </div>
{/if}
