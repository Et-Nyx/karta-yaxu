<script lang="ts">
  let {
    shortEffect,
    effect = null,
  }: { shortEffect: string; effect?: string | null } = $props();
</script>

<p class="short-effect">{shortEffect}</p>
{#if effect && effect !== shortEffect}
  <p class="long-effect">{effect}</p>
{/if}

<style>
  .short-effect {
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--theme-text-primary);
    line-height: 1.6;
    margin: 0 0 var(--spacing-3) 0;
  }

  .long-effect {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    line-height: 1.6;
    margin: 0;
    padding-top: var(--spacing-3);
    border-top: 1px dashed var(--theme-border-primary);
  }
</style>
