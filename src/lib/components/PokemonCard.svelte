<script lang="ts">
  import TypeBadge from './TypeBadge.svelte';
  import { spriteFallbacks } from '$lib/sprite-fallbacks';

  export type PokemonCardData = {
    pokeapiId: number;
    name: string;
    slug: string;
    spriteUrl: string | null;
    types: Array<{ name: string; hexColor: string | null }>;
    dexNumber?: number | null;
    isDefault?: boolean;
  };

  let { pokemon, href, highlighted = false } = $props<{
    pokemon: PokemonCardData;
    href?: string;
    highlighted?: boolean;
  }>();

  let displayName = $derived(pokemon.name ?? '???');
  let displaySlug = $derived(pokemon.slug ?? 'unknown');
  let displayTypes = $derived(pokemon.types ?? []);
  let imageUrl = $derived(
    pokemon.spriteUrl ||
    spriteFallbacks[pokemon.slug] ||
    `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/${pokemon.pokeapiId || 0}.png`
  );
  let dexNum = $derived((pokemon.dexNumber ?? pokemon.pokeapiId)?.toString().padStart(3, '0') ?? '???');
</script>

<a href={href ?? `/wiki/pokemon/${displaySlug}`} class="pokemon-card card-interactive" class:highlighted>
  <div class="card-header">
    <div class="dex-number">#{dexNum}</div>
    <div class="name">{displayName.toUpperCase()}</div>
  </div>

  <div class="sprite-container">
    <img src={imageUrl} alt={displayName} loading="lazy" />
  </div>

  <div class="types-container">
    {#each displayTypes as t}
      <TypeBadge name={t.name} hexColor={t.hexColor} size="sm" class="type-badge-flex" />
    {/each}
  </div>
</a>

<style>
  .pokemon-card {
    display: flex;
    flex-direction: column;
    padding: var(--spacing-3);
    min-width: 140px;
    min-height: 200px;
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
    max-width: 120px;
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
    border: 2px solid var(--theme-text-accent);
    box-shadow: 3px 3px 0 var(--theme-text-accent);
  }
</style>
