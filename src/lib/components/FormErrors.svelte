<script lang="ts">
  let {
    message = '',
    fieldErrors = {} as Record<string, string>,
    expanded = false,
  }: {
    message?: string;
    fieldErrors?: Record<string, string>;
    expanded?: boolean;
  } = $props();

  let entries = $derived(Object.entries(fieldErrors));
</script>

{#if message || entries.length}
  <div class="form-errors" role="alert">
    <span class="form-error">{message}</span>
    {#if expanded && entries.length > 1}
      <ul class="form-error-list">
        {#each entries as [k, m] (k)}
          <li>{m}</li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .form-errors {
    display: inline-flex;
    flex-direction: column;
    gap: 2px;
  }
  .form-error {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-error);
  }
  .form-error-list {
    list-style: none;
    padding: 0;
    margin: 0;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--color-text-error);
  }
  .form-error-list li::before {
    content: '· ';
  }
</style>
