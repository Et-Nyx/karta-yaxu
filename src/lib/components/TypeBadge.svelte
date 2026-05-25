<script lang="ts">
  let {
    name = 'normal',
    size = 'md',
    hexColor = null as string | null,
    class: className = '',
    href = `/wiki/types/${name.toLowerCase()}`
  } = $props<{
    name?: string;
    size?: 'xs' | 'sm' | 'md' | 'lg';
    hexColor?: string | null;
    class?: string;
    href?: string | null;
  }>();
</script>

{#if href}
  <a
    {href}
    class="type-badge size-{size} {className}"
    style={hexColor ? `background-color: ${hexColor}` : undefined}
  >
    {name.toUpperCase()}
  </a>
{:else}
  <span
    class="type-badge size-{size} {className} type-badge-static"
    style={hexColor ? `background-color: ${hexColor}` : undefined}
  >
    {name.toUpperCase()}
  </span>
{/if}

<style>
  .type-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    color: #ffffff;
    letter-spacing: 1px;
    text-transform: uppercase;
    font-weight: bold;
    line-height: 1;
    background-color: var(--theme-bg-tertiary);
    text-decoration: none;
    transition: transform 0.05s ease, box-shadow 0.05s ease;
  }

  .type-badge:hover {
    transform: translate(-1px, -1px);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .size-xs {
    font-size: 7px;
    padding: 1px 6px;
  }

  .size-sm {
    font-size: 8px;
    padding: 2px 8px;
  }

  .size-md {
    font-size: 10px;
    padding: 3px 10px;
  }

  .size-lg {
    font-size: 12px;
    padding: 4px 14px;
  }
</style>
