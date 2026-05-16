<script lang="ts">
  import { importActivity } from '$lib/stores/importActivity';
</script>

{#if $importActivity.isActive || $importActivity.entityType !== null}
  <div class="activity" class:active={$importActivity.isActive}>
    <div class="activity-header">
      {#if $importActivity.isActive}
        <span class="dot"></span>
        <span class="status-label">IMPORTING</span>
      {:else}
        <span class="done-mark">✓</span>
        <span class="status-label">DONE</span>
      {/if}
    </div>

    <div class="entity-type">
      {$importActivity.entityType ? $importActivity.entityType.toUpperCase() : '...'}
    </div>

    {#if $importActivity.entityType}
      <div class="counts">
        <span class="count ins">{$importActivity.inserted} ins</span>
        {#if $importActivity.updated > 0}
          <span class="count upd">{$importActivity.updated} upd</span>
        {/if}
        {#if $importActivity.skipped > 0}
          <span class="count skp">{$importActivity.skipped} skp</span>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .activity {
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    background-color: var(--theme-bg-primary);
    margin-bottom: var(--spacing-4);
    opacity: 0.7;
    transition: opacity 0.2s ease;
  }

  .activity.active {
    opacity: 1;
    border-color: var(--theme-text-primary);
  }

  .activity-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    margin-bottom: var(--spacing-2);
  }

  .status-label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .active .status-label {
    color: var(--theme-text-primary);
  }

  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    background-color: var(--theme-text-primary);
    border-radius: 50%;
    animation: pulse 1s ease-in-out infinite;
  }

  .done-mark {
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .entity-type {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-1);
  }

  .counts {
    display: flex;
    gap: var(--spacing-2);
    flex-wrap: wrap;
  }

  .count {
    font-family: var(--font-mono);
    font-size: 9px;
    padding: 1px var(--spacing-1);
  }

  .ins { color: var(--theme-text-primary); }
  .upd { color: var(--color-text-warning, #d4a017); }
  .skp { color: var(--theme-text-muted); }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>
