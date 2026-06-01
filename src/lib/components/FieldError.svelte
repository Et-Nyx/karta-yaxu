<script lang="ts">
  let {
    name,
    errors = {} as Record<string, string>,
  }: {
    name: string;
    errors?: Record<string, string>;
  } = $props();

  let msg = $derived(errors?.[name]);
</script>

{#if msg}
  <span class="field-error" role="alert">{msg}</span>
{/if}

<style>
  .field-error {
    display: block;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-error);
    margin-top: 2px;
  }
</style>
