<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    spriteDefault,
    spriteShiny,
    spriteArtwork,
    spriteArtworkShiny,
    spriteBackDefault,
    spriteBackShiny,
    pokeapiId,
    name,
    activeTab = $bindable('artwork'),
    isShiny = $bindable(false),
    inputsSnippet,
  }: {
    spriteDefault: string | null;
    spriteShiny: string | null;
    spriteArtwork: string | null;
    spriteArtworkShiny: string | null;
    spriteBackDefault: string | null;
    spriteBackShiny: string | null;
    pokeapiId: number | null;
    name: string;
    activeTab?: 'artwork' | 'front' | 'back';
    isShiny?: boolean;
    inputsSnippet?: Snippet<['artwork' | 'front' | 'back']>;
  } = $props();

  const IMAGE_TABS = [
    { key: 'artwork', label: 'Artwork' },
    { key: 'front', label: 'Front' },
    { key: 'back', label: 'Back' },
  ] as const;

  let currentSpriteUrl = $derived.by(() => {
    if (activeTab === 'artwork') {
      return isShiny
        ? (spriteArtworkShiny || (pokeapiId ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/shiny/${pokeapiId}.png` : null))
        : (spriteArtwork || (pokeapiId ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/${pokeapiId}.png` : null));
    } else if (activeTab === 'front') {
      return isShiny
        ? (spriteShiny || (pokeapiId ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/shiny/${pokeapiId}.png` : null))
        : (spriteDefault || (pokeapiId ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/${pokeapiId}.png` : null));
    } else { // 'back'
      return isShiny
        ? (spriteBackShiny || (pokeapiId ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/back/shiny/${pokeapiId}.png` : null))
        : (spriteBackDefault || (pokeapiId ? `https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/back/${pokeapiId}.png` : null));
    }
  });
</script>

<div class="sprite-viewer" class:shiny-active={isShiny}>
  <!-- Tab navigation -->
  <div class="tab-nav">
    {#each IMAGE_TABS as tab}
      <button
        type="button"
        class="tab-btn"
        class:active={activeTab === tab.key}
        onclick={() => activeTab = tab.key}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Image preview container -->
  <div class="image-area">
    {#if currentSpriteUrl}
      <img src={currentSpriteUrl} alt={name} class="viewer-img" />
    {:else}
      <div class="no-sprite">NO SPRITE</div>
    {/if}
    <button type="button" class="shiny-toggle" onclick={() => isShiny = !isShiny}>
      {isShiny ? '◼ NORMAL' : '✦ SHINY'}
    </button>
  </div>

  <!-- Nested inputs for the editor -->
  {#if inputsSnippet}
    <div class="inputs-area">
      {@render inputsSnippet(activeTab)}
    </div>
  {/if}
</div>

<style>
  .sprite-viewer {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    display: flex;
    flex-direction: column;
    transition: background-color 0.15s ease;
    flex: 1;
    overflow: hidden;
  }

  .tab-nav {
    display: flex;
    flex-wrap: wrap;
    border-bottom: 3px solid var(--theme-border-primary);
    background-color: var(--theme-bg-tertiary);
  }

  .tab-btn {
    padding: var(--spacing-3) var(--spacing-4);
    background: none;
    border: none;
    border-right: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-inverse);
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.1s ease;
  }

  .tab-btn:last-child {
    border-right: none;
  }

  .tab-btn:hover {
    opacity: 0.85;
  }

  .tab-btn.active {
    opacity: 1;
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
    border-bottom: 3px solid var(--theme-bg-secondary);
    margin-bottom: -3px;
  }

  .image-area {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: var(--spacing-6);
    flex: 1;
    min-height: 240px;
    transition: background-color 0.15s ease;
  }

  .sprite-viewer.shiny-active .image-area {
    background-color: color-mix(in srgb, var(--theme-bg-secondary) 85%, #b39ddb 15%);
  }

  .viewer-img {
    max-width: 100%;
    max-height: 240px;
    object-fit: contain;
  }

  .no-sprite {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .shiny-toggle {
    position: absolute;
    bottom: var(--spacing-2);
    right: var(--spacing-2);
    font-family: var(--font-display);
    font-size: 8px;
    padding: 4px 8px;
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-secondary);
    border: 2px solid var(--theme-border-primary);
    cursor: pointer;
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    transition: all 0.05s ease;
  }

  .shiny-toggle:hover {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
    transform: translate(-1px, -1px);
  }

  .inputs-area {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
    padding: var(--spacing-3);
    border-top: 2px dashed var(--theme-border-primary);
    background-color: var(--theme-bg-primary);
  }
</style>
