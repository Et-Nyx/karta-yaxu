<script lang="ts">
  import TypeBadge from './TypeBadge.svelte';

  export type PokemonCardData = {
    pokeapiId: number;
    name: string;
    slug: string;
    spriteUrl: string | null;
    types: Array<{ name: string; hexColor: string | null }>;
    dexNumber?: number | null;
    isDefault?: boolean;
  };

  let {
    pokemon,
    href = undefined,
    onclick = null,
    highlighted = false,
    dashed = false,
    static: isStatic = false
  } = $props<{
    pokemon: PokemonCardData;
    href?: string | null;
    onclick?: ((e: MouseEvent) => void) | null;
    highlighted?: boolean;
    dashed?: boolean;
    static?: boolean;
  }>();

  let displayName = $derived(pokemon.name ?? '???');
  let displaySlug = $derived(pokemon.slug ?? 'unknown');
  let displayTypes = $derived(pokemon.types ?? []);
  let imageUrl = $derived(
    pokemon.spriteUrl ||
    `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/${pokemon.pokeapiId || 0}.png`
  );
  let dexNum = $derived((pokemon.dexNumber ?? pokemon.pokeapiId)?.toString().padStart(3, '0') ?? '???');

  let finalHref = $derived(
    isStatic
      ? null
      : href !== undefined
        ? href
        : onclick
          ? null
          : `/wiki/pokemon/${displaySlug}`
  );
</script>

{#snippet cardContent()}
  <div class="card-header">
    <div class="dex-number">#{dexNum}</div>
    <div class="name">{displayName.toUpperCase()}</div>
  </div>

  <div class="sprite-container">
    <img src={imageUrl} alt={displayName} loading="lazy" />
  </div>

  <div class="types-container">
    {#each displayTypes as t}
      <TypeBadge name={t.name} hexColor={t.hexColor} size="sm" class="type-badge-flex" href={null} />
    {/each}
  </div>
{/snippet}

{#if finalHref}
  <a href={finalHref} class="pokemon-card card-interactive" class:highlighted class:dashed-border={dashed}>
    {@render cardContent()}
  </a>
{:else if onclick}
  <button type="button" {onclick} class="pokemon-card card-interactive card-button" class:highlighted class:dashed-border={dashed}>
    {@render cardContent()}
  </button>
{:else}
  <div class="pokemon-card card-static" class:highlighted class:dashed-border={dashed}>
    {@render cardContent()}
  </div>
{/if}

<style>
  .dashed-border {
    border-style: dashed !important;
    position: relative;
  }

  .dashed-border::after {
    content: "EDIT RELATION";
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: rgba(15, 56, 15, 0.9); /* Retro deep forest green overlay */
    color: var(--theme-text-accent);
    font-family: var(--font-display);
    font-size: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    line-height: 1.5;
    padding: var(--spacing-3);
    opacity: 0;
    transition: opacity 0.1s steps(2); /* Crisp retro step-fade transition */
    z-index: 20;
    pointer-events: none;
    border: 3px dashed var(--theme-text-accent);
    margin: -3px; /* Align perfectly with outer border */
    text-shadow: 1px 1px 0px #0f380f;
  }

  .dashed-border:hover::after {
    opacity: 1;
  }

  .pokemon-card {
    display: flex;
    flex-direction: column;
    padding: var(--spacing-3);
    min-width: 140px;
    min-height: 200px;
    position: relative;
  }

  .card-button {
    appearance: none;
    border: 3px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    box-shadow: var(--shadow-card);
    text-align: left;
    cursor: pointer;
    font: inherit;
    width: 100%;
    margin: 0;
  }

  .card-static {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--spacing-2);
  }

  .dex-number {
    font-family: var(--font-display);
    font-size: 8px;
    background-color: var(--theme-border-primary);
    color: var(--theme-bg-primary);
    padding: 2px 4px;
  }

  .name {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    font-weight: bold;
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    margin-left: var(--spacing-2);
  }

  .sprite-container {
    flex: 1;
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 120px;
    image-rendering: pixelated; /* Crucial for gameboy feel */
  }

  .sprite-container img {
    width: 80%;
    height: auto;
    max-width: 96px;
    object-fit: contain;
    transition: transform 0.2s ease;
  }

  .pokemon-card:hover .sprite-container img {
    transform: scale(1.1);
  }

  .types-container {
    display: flex;
    gap: var(--spacing-2);
    justify-content: center;
    margin-top: var(--spacing-2);
  }

  :global(.type-badge-flex) {
    flex: 1;
    text-align: center;
  }

  .pokemon-card.highlighted {
    border-color: var(--theme-text-accent);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }
</style>
