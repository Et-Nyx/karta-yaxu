<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import MoveCategoryIcon from '$lib/components/MoveCategoryIcon.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import InfoBox from '$lib/components/InfoBox.svelte';
  import VersionSelector from '$lib/components/VersionSelector.svelte';
  import FlavorTextBox from '$lib/components/FlavorTextBox.svelte';
  import EffectText from '$lib/components/EffectText.svelte';

  let { data } = $props();

  let m = $derived(data.move);

  let selectedVersionGroupId = $state<number | null>(null);

  $effect(() => {
    // Reactively reset selectedVersionGroupId on page transition
    data;
    selectedVersionGroupId = null;
  });

  $effect(() => {
    if (selectedVersionGroupId === null && data.flavorTexts.length > 0) {
      selectedVersionGroupId = data.flavorTexts[data.flavorTexts.length - 1].versionGroupId;
    }
  });

  let selectedFlavorText = $derived(
    data.flavorTexts.find(f => f.versionGroupId === selectedVersionGroupId)?.flavorText ?? null
  );

  let searchQuery = $state('');

  let filteredLearners = $derived(
    data.learners.filter(p =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.pokeapiId?.toString() === searchQuery
    )
  );
</script>

<svelte:head>
  <title>{m.name.toUpperCase()} | Karta Yaxu</title>
</svelte:head>

<div class="move-detail-page">
  <!-- Cabecalho do Move -->
  <header class="detail-header">
    <div class="title-group">
      <h1>{m.name.toUpperCase()}</h1>
      <MoveCategoryIcon category={m.damageClass || 'physical'} size="lg" />
      {#if m.typeName}
        <TypeBadge name={m.typeName} hexColor={m.typeHexColor} size="lg" />
      {/if}
    </div>

    <div class="stats-row">
      <div class="stat">
        <span class="label">POWER</span>
        <span class="value">{m.power !== null ? m.power : '-'}</span>
      </div>
      <div class="stat">
        <span class="label">ACCURACY</span>
        <span class="value">{m.accuracy !== null ? m.accuracy + '%' : '-'}</span>
      </div>
      <div class="stat">
        <span class="label">PP</span>
        <span class="value">{m.pp !== null ? m.pp : '-'}</span>
      </div>
      <div class="stat">
        <span class="label">PRIORITY</span>
        <span class="value">{m.priority}</span>
      </div>
      <div class="stat">
        <span class="label">TARGET</span>
        <span class="value">{m.target ? m.target.toUpperCase().replace('-', ' ') : '-'}</span>
      </div>
    </div>
  </header>

  {#if data.shortEffect || data.flavorTexts.length > 0}
    <div class="detail-grid">
      {#if data.flavorTexts.length > 0}
        <div class="left-col">
          <VersionSelector
            bind:value={selectedVersionGroupId}
            options={data.flavorTexts.map(f => ({
              value: f.versionGroupId,
              label: f.versionGroupName.toUpperCase().replace(/-/g, ' ')
            }))}
          />
          {#if selectedFlavorText}
            <FlavorTextBox text={selectedFlavorText} />
          {/if}
        </div>
      {/if}

      {#if data.shortEffect}
        <div class="right-col">
          <InfoBox title="EFFECT">
            <EffectText shortEffect={data.shortEffect} effect={data.effect} />
          </InfoBox>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Lista de Pokémon que aprendem -->
  <section class="learners-section">
    <div class="section-header">
      <h2>LEARNED BY</h2>
      <SearchBar bind:value={searchQuery} placeholder="Filter Pokémon..." count={filteredLearners.length} total={data.learners.length} />
    </div>

    <div class="pokemon-grid">
      {#each filteredLearners as p (p.id)}
        <PokemonCard
          pokemon={{
            pokeapiId: p.pokeapiId || 0,
            name: p.name,
            slug: p.slug,
            spriteUrl: p.spriteDefault,
            types: p.types
          }}
        />
      {/each}
    </div>

    {#if filteredLearners.length === 0}
      <div class="empty-state">
        <p>No Pokémon found.</p>
      </div>
    {/if}
  </section>
</div>

<style>
  .move-detail-page {
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
    gap: var(--spacing-6);
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    flex-wrap: wrap;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .stats-row {
    display: flex;
    gap: var(--spacing-8);
    flex-wrap: wrap;
    border-top: 2px dashed var(--theme-border-primary);
    padding-top: var(--spacing-4);
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .stat .label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .stat .value {
    font-family: var(--font-mono);
    font-size: 20px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  .learners-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
    font-size: 18px;
  }

  .pokemon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-4);
  }

  .empty-state {
    text-align: center;
    padding: var(--spacing-8);
    font-family: var(--font-mono);
    color: var(--theme-text-secondary);
  }

  .detail-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
    align-items: start;
  }

  .left-col, .right-col {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  @media (max-width: 768px) {
    h1 { font-size: 18px; }

    .detail-header {
      padding: var(--spacing-4);
      gap: var(--spacing-4);
    }

    .stat .value { font-size: 16px; }

    .detail-grid { grid-template-columns: minmax(0, 1fr); }
    .left-col, .right-col { min-width: 0; }
  }
</style>
