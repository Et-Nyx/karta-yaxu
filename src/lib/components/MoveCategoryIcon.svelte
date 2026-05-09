<script lang="ts">
  let {
    category = 'physical',
    size = 'md', // sm, md, lg
    class: className = ''
  } = $props();

  const iconUrls: Record<string, string> = {
    physical: 'https://archives.bulbagarden.net/media/upload/b/b4/PhysicalIC_SV.png',
    special: 'https://archives.bulbagarden.net/media/upload/5/5b/SpecialIC_SV.png',
    status: 'https://archives.bulbagarden.net/media/upload/e/e0/StatusIC_SV.png'
  };

  let iconUrl = $derived(iconUrls[category.toLowerCase()] || iconUrls.physical);

  let sizeClass = $derived({
    sm: 'move-category-icon-sm',
    md: 'move-category-icon-md',
    lg: 'move-category-icon-lg'
  }[size] || 'move-category-icon-md');
</script>

<img
  src={iconUrl}
  alt="{category} category"
  class="move-category-icon {sizeClass} {className}"
  loading="lazy"
/>

<style>
  .move-category-icon {
    display: inline-block;
    object-fit: contain;
    image-rendering: pixelated;
    vertical-align: middle;
    transition: transform 0.1s ease;
    filter: var(--filter-move-icon, none);
  }

  .move-category-icon-sm {
    width: 16px;
    height: 13px;
  }

  .move-category-icon-md {
    width: 24px;
    height: 19px;
  }

  .move-category-icon-lg {
    width: 32px;
    height: 26px;
  }

  /* Slight scale on parent hover when inside an interactive element */
  :global(.card-interactive:hover) .move-category-icon {
    transform: scale(1.15);
  }
</style>
