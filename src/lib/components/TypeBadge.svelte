<script lang="ts">
  let {
    name = 'normal',
    size = 'md',
    hexColor = null as string | null,
    class: className = ''
  } = $props();
</script>

<a
  href="/wiki/types/{name.toLowerCase()}"
  class="type-badge size-{size} {className}"
  style={hexColor ? `background-color: ${hexColor}` : undefined}
>
  {name.toUpperCase()}
</a>

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
