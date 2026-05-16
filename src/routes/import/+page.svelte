<script lang="ts">
  import ImportStatusGrid from '$lib/components/ImportStatusGrid.svelte';
  import ImportPanel from '$lib/components/ImportPanel.svelte';
  import ImportLog from '$lib/components/ImportLog.svelte';

  let { data } = $props();
</script>

<svelte:head>
  <title>Import | Karta Yaxu</title>
</svelte:head>

<div class="import-page">
  <div class="page-header">
    <h2>PokeAPI Import Dashboard</h2>
    <p>Database seeded on-demand. Manual edits are protected from overwriting.</p>
  </div>

  <section>
    <ImportStatusGrid status={data.status} />
  </section>

  <section>
    <ImportPanel regions={data.regions} />
  </section>

  <section>
    <ImportLog logs={data.logs} />
  </section>
</div>

<style>
  .import-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
  }

  .page-header p {
    color: var(--theme-text-secondary);
    font-size: 14px;
    margin: 0;
  }
</style>
