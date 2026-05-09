<script lang="ts">
  let { 
    typeName = 'normal', 
    size = 'md', // sm, md, lg
    class: className = ''
  } = $props();

  // Map type name to PokeAPI type ID for icon URL
  const typeIdMap: Record<string, number> = {
    normal: 1, fighting: 2, flying: 3, poison: 4, ground: 5,
    rock: 6, bug: 7, ghost: 8, steel: 9, fire: 10,
    water: 11, grass: 12, electric: 13, psychic: 14, ice: 15,
    dragon: 16, dark: 17, fairy: 18, stellar: 19
  };

  let typeId = $derived(typeIdMap[typeName.toLowerCase()] || 1);
  let iconUrl = $derived(`https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/types/generation-ix/scarlet-violet/small/${typeId}.png`);

  let sizeClass = $derived({
    sm: 'type-icon-sm',
    md: 'type-icon-md',
    lg: 'type-icon-lg'
  }[size] || 'type-icon-md');
</script>

<img 
  src={iconUrl} 
  alt="{typeName} type icon"
  class="type-icon {sizeClass} {className}"
  loading="lazy"
/>

<style>
  .type-icon {
    display: inline-block;
    object-fit: contain;
    image-rendering: pixelated;
    vertical-align: middle;
    transition: transform 0.1s ease;
  }

  .type-icon-sm {
    width: 20px;
    height: 20px;
  }

  .type-icon-md {
    width: 32px;
    height: 32px;
  }

  .type-icon-lg {
    width: 48px;
    height: 48px;
  }

  /* Slight scale on parent hover when the icon is inside an interactive element */
  :global(.card-interactive:hover) .type-icon {
    transform: scale(1.15);
  }
</style>
