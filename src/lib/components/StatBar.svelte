<script lang="ts">
  let { 
    name = 'hp', 
    value = 0,
    max = 255 
  } = $props();

  // Mapeia nomes das PokeAPI ("hp", "attack", "special-attack") para os nossos tokens CSS
  let cssName = $derived(() => {
    switch(name) {
      case 'hp': return 'hp';
      case 'attack': return 'atk';
      case 'defense': return 'def';
      case 'special-attack': return 'spa';
      case 'special-defense': return 'spd';
      case 'speed': return 'spe';
      default: return 'hp';
    }
  });

  let percentage = $derived(Math.min(100, (value / max) * 100));
</script>

<div class="stat-row">
  <div class="stat-label">{name.replace('special-', 'sp. ').toUpperCase()}</div>
  <div class="stat-value">{value.toString().padStart(3, '0')}</div>
  <div class="stat-bar-container">
    <div 
      class="stat-bar-fill" 
      style="width: {percentage}%; background-color: var(--color-stat-{cssName()});"
    ></div>
  </div>
</div>

<style>
  .stat-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    margin-bottom: var(--spacing-2);
  }

  .stat-label {
    width: 80px;
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    text-align: right;
  }

  .stat-value {
    width: 30px;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: bold;
    color: var(--theme-text-primary);
  }

  .stat-bar-container {
    flex: 1;
    height: 12px;
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px rgba(0,0,0,0.1);
  }

  .stat-bar-fill {
    height: 100%;
    border-right: 1px solid var(--theme-border-primary);
  }
</style>
