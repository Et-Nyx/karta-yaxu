<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import MoveCategoryIcon from '$lib/components/MoveCategoryIcon.svelte';
  import StatBar from '$lib/components/StatBar.svelte';
  import EvolutionTree from '$lib/components/EvolutionTree.svelte';

  let { data } = $props();

  let p = $derived(data.pokemon);
  let s = $derived(data.species);

  // Usa proxy local ou a URL original
  let spriteUrl = $derived(p.spriteArtwork || p.spriteDefault || `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/${p.pokeapiId}.png`);
  
  let totalStats = $derived(data.stats.reduce((acc, stat) => acc + stat.baseStat, 0));

  // Version Mechanics
  // Determine which versions actually have data for this pokemon
  const versionIdsWithData = new Set([
    ...data.flavorTexts.map(f => f.versionGroupId),
    ...data.learnset.map(l => l.versionGroupId)
  ]);
  
  let validVersions = $derived(data.availableVersions.filter(v => versionIdsWithData.has(v.id)));
  let selectedVersionId = $state<number | null>(null);

  // Initialize selected version to the latest one available if not set
  $effect(() => {
    if (selectedVersionId === null && validVersions.length > 0) {
      selectedVersionId = validVersions[validVersions.length - 1].id;
    }
  });

  let currentFlavorText = $derived(() => {
    if (!selectedVersionId) return 'No Pokédex entry available.';
    const entry = data.flavorTexts.find(f => f.versionGroupId === selectedVersionId);
    return entry ? entry.flavorText.replace(/\f|\n/g, ' ') : 'No Pokédex entry available for this version.';
  });

  let currentLearnset = $derived(() => {
    if (!selectedVersionId) return [];
    return data.learnset.filter(l => l.versionGroupId === selectedVersionId);
  });
</script>

<svelte:head>
  <title>{p.name.toUpperCase()} | Karta Yaxu</title>
</svelte:head>

<div class="pokemon-detail">
  <!-- Cabecalho -->
  <header class="detail-header">
    <div class="dex-num">#{p.pokeapiId.toString().padStart(3, '0')}</div>
    <h1>{p.name.toUpperCase()}</h1>
    <div class="types">
      {#each data.types as t}
        <TypeBadge name={t.name} size="lg" />
      {/each}
    </div>
  </header>

  <div class="content-grid">
    <!-- Esquerda: Imagem e Dados Básicos -->
    <div class="left-col">
      <div class="image-box">
        <img src={spriteUrl} alt={p.name} />
      </div>

      {#if validVersions.length > 0}
        <div class="version-selector-container">
          <label for="versionSelect">VERSION:</label>
          <select id="versionSelect" bind:value={selectedVersionId} class="version-select">
            {#each validVersions as v}
              <option value={v.id}>{v.name.toUpperCase().replace('-', ' ')}</option>
            {/each}
          </select>
        </div>

        <div class="flavor-text-box">
          <p>{currentFlavorText()}</p>
        </div>
      {/if}

      <div class="info-box">
        <h3>DATA</h3>
        <ul class="data-list">
          <li>
            <span class="label">HEIGHT</span>
            <span class="value">{p.height / 10} m</span>
          </li>
          <li>
            <span class="label">WEIGHT</span>
            <span class="value">{p.weight / 10} kg</span>
          </li>
          <li>
            <span class="label">HABITAT</span>
            <span class="value">{s.habitat ? s.habitat.toUpperCase() : 'UNKNOWN'}</span>
          </li>
          <li>
            <span class="label">GROWTH</span>
            <span class="value">{s.growthRate ? s.growthRate.toUpperCase() : 'UNKNOWN'}</span>
          </li>
        </ul>
      </div>

      <div class="info-box">
        <h3>ABILITIES</h3>
        <ul class="ability-list">
          {#each data.abilities as a}
            <li class:hidden-ability={a.isHidden}>
              <a href="/wiki/abilities/{a.slug}" class="ability-link">
                <span class="name">{a.name.toUpperCase()}</span>
                {#if a.isHidden}
                  <span class="badge">HIDDEN</span>
                {/if}
              </a>
            </li>
          {/each}
        </ul>
      </div>
    </div>

    <!-- Direita: Base Stats -->
    <div class="right-col">
      <div class="info-box stats-box">
        <h3>BASE STATS</h3>
        <div class="stats-container">
          {#each data.stats as stat}
            <StatBar name={stat.statName} value={stat.baseStat} max={255} />
          {/each}
          
          <div class="stat-total">
            <span class="label">TOTAL:</span>
            <span class="value">{totalStats}</span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Evolution Tree -->
  {#if data.evolutionNodes && data.evolutionNodes.length > 0}
    <div class="info-box evolution-box">
      <h3>EVOLUTION CHAIN</h3>
      <EvolutionTree nodes={data.evolutionNodes} edges={data.evolutionEdges} />
    </div>
  {/if}

  <!-- Learnset -->
  {#if validVersions.length > 0}
    <div class="info-box learnset-box">
      <h3>LEARNSET ({validVersions.find(v => v.id === selectedVersionId)?.name.toUpperCase().replace('-', ' ')})</h3>
      
      {#if currentLearnset().length > 0}
        <div class="learnset-table-wrapper">
          <table class="learnset-table">
            <thead>
              <tr>
                <th>LVL / METHOD</th>
                <th>MOVE</th>
                <th>TYPE</th>
                <th>CAT</th>
              </tr>
            </thead>
            <tbody>
              {#each currentLearnset() as move}
                <tr>
                  <td class="method">
                    {move.methodName === 'level-up' ? `Lvl ${move.level}` : move.methodName.toUpperCase()}
                  </td>
                  <td class="move-name"><a href="/wiki/moves/{move.moveSlug}">{move.moveName.toUpperCase().replace(/-/g, ' ')}</a></td>
                  <td><MoveCategoryIcon category={move.damageClass} size="sm" /></td>
                  <td><TypeBadge name={move.typeName} size="sm" /></td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <p class="empty-text">No moves learned in this version.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .pokemon-detail {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    padding: var(--spacing-4) var(--spacing-6);
    box-shadow: 4px 4px 0px rgba(0,0,0,0.1);
  }

  .dex-num {
    font-family: var(--font-display);
    font-size: 16px;
    color: var(--theme-text-secondary);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
    flex: 1;
  }

  .types {
    display: flex;
    gap: var(--spacing-2);
  }

  .content-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
  }

  @media (max-width: 768px) {
    .content-grid {
      grid-template-columns: 1fr;
    }
  }

  .left-col, .right-col {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .image-box {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    display: flex;
    justify-content: center;
    align-items: center;
    padding: var(--spacing-6);
    height: 300px;
  }

  .image-box img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }

  .info-box {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-4);
  }

  .info-box h3 {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-4) 0;
    border-bottom: 2px solid var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
  }

  .evolution-box {
    margin-top: var(--spacing-4);
  }

  .data-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .data-list li {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 14px;
  }

  .data-list .label {
    color: var(--theme-text-secondary);
  }

  .data-list .value {
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  .ability-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .ability-list li {
    display: flex;
    align-items: center;
  }

  .ability-list li.hidden-ability .ability-link {
    color: var(--theme-text-secondary);
  }

  .ability-link {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-2);
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
    text-decoration: none;
    border: 3px solid transparent;
    padding: 2px 6px;
    margin: -2px -6px;
    transition: all 0.05s ease;
  }

  .ability-link:hover {
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0px var(--theme-text-accent);
    transform: translate(-2px, -2px);
    color: var(--theme-text-accent);
  }

  .ability-list .badge {
    font-family: var(--font-display);
    font-size: 8px;
    background-color: var(--theme-bg-tertiary);
    color: var(--theme-text-inverse);
    padding: 2px 4px;
    border-radius: 2px;
  }

  .stats-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .stat-total {
    display: flex;
    justify-content: space-between;
    margin-top: var(--spacing-4);
    padding-top: var(--spacing-4);
    border-top: 2px dashed var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
  }

  .version-selector-container {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .version-selector-container label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
  }

  .version-select {
    flex: 1;
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 10px;
  }

  .flavor-text-box {
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.5;
    color: var(--theme-text-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .learnset-box {
    margin-top: var(--spacing-4);
  }

  .learnset-table-wrapper {
    overflow-x: auto;
  }

  .learnset-table {
    width: 100%;
    border-collapse: collapse;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .learnset-table th, .learnset-table td {
    padding: var(--spacing-2) var(--spacing-3);
    text-align: left;
    border-bottom: 1px dashed var(--theme-border-primary);
  }

  .learnset-table th {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  .learnset-table .method {
    font-family: var(--font-display);
    font-size: 10px;
  }

  .learnset-table .move-name a {
    color: var(--theme-text-primary);
    text-decoration: none;
    font-weight: bold;
    border: 3px solid transparent;
    padding: 2px 6px;
    margin: -2px -6px;
    display: inline-block;
    transition: all 0.05s ease;
  }

  .learnset-table .move-name a:hover {
    border-color: var(--theme-text-accent);
    box-shadow: 2px 2px 0px var(--theme-text-accent);
    transform: translate(-2px, -2px);
    color: var(--theme-text-accent);
  }

  .empty-text {
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }
</style>
