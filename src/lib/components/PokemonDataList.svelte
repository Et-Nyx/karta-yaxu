<script lang="ts">
  let {
    height = $bindable(),
    weight = $bindable(),
    captureRate,
    genderText,
    eggGroups,
    habitat,
    growthRate,
    happiness,
    pokemonId,
    isDefault,
    speciesSlug,
    editable = false,
  }: {
    height: number;
    weight: number;
    captureRate: number | null;
    genderText: string;
    eggGroups: string;
    habitat: string | null;
    growthRate: string | null;
    happiness: number | null;
    pokemonId?: number;
    isDefault?: boolean;
    speciesSlug?: string | null;
    editable?: boolean;
  } = $props();

  // Create UI bindings that sync with the decimeters/hectograms db values
  let uiHeight = $state(height / 10);
  let uiWeight = $state(weight / 10);

  // Keep height/weight in sync with parent db values
  $effect(() => {
    // When parent db value changes, update UI
    uiHeight = height / 10;
  });
  $effect(() => {
    uiHeight = Math.max(0, uiHeight);
    height = Math.round(uiHeight * 10);
  });

  $effect(() => {
    // When parent db value changes, update UI
    uiWeight = weight / 10;
  });
  $effect(() => {
    uiWeight = Math.max(0, uiWeight);
    weight = Math.round(uiWeight * 10);
  });
</script>

<ul class="data-list">
  <li>
    <span class="label">HEIGHT</span>
    {#if editable}
      <div class="data-input-wrap">
        <input class="data-input" type="number" min="0" step="0.1" bind:value={uiHeight} />
        <span class="data-unit">m</span>
      </div>
    {:else}
      <span class="value">{(height / 10).toFixed(1)} m</span>
    {/if}
  </li>

  <li>
    <span class="label">WEIGHT</span>
    {#if editable}
      <div class="data-input-wrap">
        <input class="data-input" type="number" min="0" step="0.1" bind:value={uiWeight} />
        <span class="data-unit">kg</span>
      </div>
    {:else}
      <span class="value">{(weight / 10).toFixed(1)} kg</span>
    {/if}
  </li>

  {#if captureRate !== null}
    <li>
      <span class="label">CAPTURE</span>
      <span class="value">{captureRate}</span>
    </li>
  {/if}

  <li>
    <span class="label">GENDER</span>
    <span class="value">{genderText}</span>
  </li>

  <li>
    <span class="label">EGG GROUP{#if !editable && eggGroups.includes(',')}S{/if}</span>
    <span class="value">{eggGroups}</span>
  </li>

  {#if habitat}
    <li>
      <span class="label">HABITAT</span>
      <span class="value">{habitat.toUpperCase()}</span>
    </li>
  {/if}

  {#if growthRate}
    <li>
      <span class="label">GROWTH</span>
      <span class="value">{growthRate.toUpperCase()}</span>
    </li>
  {/if}

  {#if happiness !== null}
    <li>
      <span class="label">HAPPINESS</span>
      <span class="value">{happiness}</span>
    </li>
  {/if}

  {#if pokemonId !== undefined}
    <li>
      <span class="label">ID</span>
      <span class="value">{pokemonId}</span>
    </li>
  {/if}

  {#if isDefault !== undefined}
    <li>
      <span class="label">IS DEFAULT</span>
      <span class="value">{isDefault ? 'YES' : 'NO'}</span>
    </li>
  {/if}
</ul>

{#if editable && speciesSlug}
  <a href="/editor/species/{speciesSlug}" class="edit-species-btn">
    ✎ EDIT SPECIES DETAILS
  </a>
{/if}

<style>
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
    align-items: center;
    font-family: var(--font-mono);
    font-size: 14px;
    min-height: 24px;
  }

  .data-list .label {
    color: var(--theme-text-secondary);
  }

  .data-list .value {
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  .data-input-wrap {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .data-input {
    width: 60px;
    height: 24px;
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: bold;
    text-align: center;
    outline: none;
    box-sizing: border-box;
    padding: 0;
    -moz-appearance: textfield;
  }

  .data-input::-webkit-outer-spin-button,
  .data-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .data-input:focus {
    border-color: var(--theme-text-accent);
  }

  .data-unit {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-secondary);
  }

  .edit-species-btn {
    display: block;
    margin-top: var(--spacing-4);
    text-align: center;
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2);
    text-decoration: none;
    box-shadow: var(--shadow-card-subtle);
    transition: all 0.1s ease;
  }

  .edit-species-btn:hover {
    background-color: var(--theme-bg-hover);
    transform: translate(-1px, -1px);
    box-shadow: var(--shadow-card-hover);
  }

  @media (max-width: 768px) {
    .data-list li {
      font-size: 11px;
    }
  }
</style>
