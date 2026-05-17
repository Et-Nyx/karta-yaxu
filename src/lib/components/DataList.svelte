<script lang="ts">
  let { items }: { items: { label: string; value: string }[] } = $props();
</script>

<ul class="data-list">
  {#each items as item}
    <li>
      <span class="label">{item.label}</span>
      <span class="value">{item.value}</span>
    </li>
  {/each}
</ul>

<style>
  .data-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .data-list li {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 14px;
  }

  .data-list .label { color: var(--theme-text-secondary); }
  .data-list .value { color: var(--theme-text-primary); font-weight: bold; }
</style>
