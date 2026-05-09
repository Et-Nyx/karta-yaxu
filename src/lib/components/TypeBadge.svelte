<script lang="ts">
  let {
    name = 'normal',
    size = 'md', // sm, md, lg
    class: className = ''
  } = $props();

  const typeColors: Record<string, string> = {
    normal: '#9FA19F', fire: '#E62829', water: '#2980EF', electric: '#FAC000',
    grass: '#3FA129', ice: '#3DD8FF', fighting: '#FF8000', poison: '#9141CB',
    ground: '#915121', flying: '#81B9EF', psychic: '#EF4179', bug: '#91A119',
    rock: '#AFA981', ghost: '#704170', dragon: '#5060E1', dark: '#4F3F3D',
    steel: '#60A1B8', fairy: '#EF70EF', stellar: '#83CFC5'
  };

  let bgColor = $derived(typeColors[name.toLowerCase()] || typeColors.normal);
</script>

<span
  class="type-badge size-{size} {className}"
  style="background-color: {bgColor}"
>
  {name.toUpperCase()}
</span>

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
