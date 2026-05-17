<script lang="ts">
  let { text }: { text: string } = $props();
</script>

<div class="flavor-text-box">
  <p>{text}</p>
</div>

<style>
  .flavor-text-box {
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.5;
    color: var(--theme-text-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .flavor-text-box p { margin: 0; }

  @media (max-width: 768px) {
    .flavor-text-box { font-size: 11px; line-height: 1.4; }
  }
</style>
