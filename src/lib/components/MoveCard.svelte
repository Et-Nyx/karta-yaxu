<script lang="ts">
  import TypeBadge from './TypeBadge.svelte';
  import MoveCategoryIcon from './MoveCategoryIcon.svelte';

  let {
    name,
    slug,
    typeName,
    typeHexColor = null,
    damageClass,
    power,
    accuracy,
    pp,
    method = undefined,
  }: {
    name: string;
    slug: string;
    typeName: string;
    typeHexColor?: string | null;
    damageClass: 'physical' | 'special' | 'status';
    power: number | null;
    accuracy: number | null;
    pp: number | null;
    method?: string;
  } = $props();
</script>

<a href="/wiki/moves/{slug}" class="move-card card-interactive">
  <div class="move-info">
    {#if method !== undefined}
      <div class="move-method">{method}</div>
    {/if}
    <div class="move-name">{name.toUpperCase().replace(/-/g, ' ')}</div>
    <MoveCategoryIcon category={damageClass} size="md" />
    <TypeBadge name={typeName} hexColor={typeHexColor} size="sm" />
  </div>

  <div class="move-stats">
    <div class="stat">
      <span class="label">PWR</span>
      <span class="value">{power !== null ? power : '—'}</span>
    </div>
    <div class="stat">
      <span class="label">ACC</span>
      <span class="value">{accuracy !== null ? accuracy + '%' : '—'}</span>
    </div>
    <div class="stat">
      <span class="label">PP</span>
      <span class="value">{pp !== null ? pp : '—'}</span>
    </div>
  </div>
</a>

<style>
  .move-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-3) var(--spacing-4);
  }

  .move-info {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    flex: 1;
  }

  .move-method {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    width: 48px;
    text-align: right;
    flex-shrink: 0;
  }

  .move-name {
    font-family: var(--font-display);
    font-size: 12px;
    color: var(--theme-text-primary);
    font-weight: bold;
    width: 200px;
  }

  .move-stats {
    display: flex;
    gap: var(--spacing-5);
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 50px;
  }

  .stat .label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    margin-bottom: 2px;
  }

  .stat .value {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
    font-weight: bold;
  }

  @media (max-width: 768px) {
    .move-card {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--spacing-4);
    }

    .move-info {
      width: 100%;
      flex-wrap: wrap;
    }

    .move-stats {
      width: 100%;
      justify-content: space-between;
    }
  }
</style>
