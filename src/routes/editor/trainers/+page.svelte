<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.npcs.filter(n => n.username.toLowerCase().includes(query.toLowerCase()))
  );

  function formatDate(ts: Date | null): string {
    if (!ts) return '—';
    return new Date(ts).toLocaleDateString('pt-BR');
  }
</script>

<svelte:head>
  <title>Edit Trainers | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>TRAINERS (NPC)</h2>
    </div>
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW NPC'}
    </button>
  </header>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE NPC</h3>
      <form action="?/create" method="post" use:enhance class="create-form">
        <div class="field">
          <label for="c-username">USERNAME</label>
          <input id="c-username" name="username" type="text" maxlength="30" required
            placeholder="e.g. Brock">
        </div>
        <div class="form-actions">
          <button type="submit" class="save-btn">CREATE</button>
          {#if form?.createError}
            <span class="form-error">{form.createError}</span>
          {/if}
        </div>
      </form>
    </section>
  {/if}

  <SearchBar bind:value={query} placeholder="Search NPCs..." count={filtered.length} total={data.total} />

  <div class="entity-table">
    <div class="table-header">
      <span>USERNAME</span>
      <span>CREATED</span>
      <span></span>
      <span></span>
    </div>
    {#each filtered as npc (npc.id)}
      <div class="table-row">
        <span class="row-name">{npc.username}</span>
        <span class="row-date">{formatDate(npc.createdAt)}</span>
        <a href="/editor/trainers/{npc.username}" class="edit-link">EDIT →</a>
        <form action="?/delete" method="post" use:enhance>
          <input type="hidden" name="id" value={npc.id}>
          <button type="submit" class="delete-btn">DELETE</button>
        </form>
      </div>
    {/each}
  </div>
</div>

<style>
  .entity-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
  }

  .page-header h2 { color: var(--theme-text-primary); margin: 0; }

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .new-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    cursor: pointer;
  }
  .new-btn:hover { border-color: var(--theme-text-accent); color: var(--theme-text-accent); }

  .create-section {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-3) 0;
  }

  .create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--spacing-3);
  }

  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }

  .field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .field input {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field input:focus { outline: none; border-color: var(--theme-text-accent); }

  .form-actions {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .save-btn {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
  }
  .save-btn:hover { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }

  .form-error { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-error); }

  .entity-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow-x: auto;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 1fr 120px 60px 70px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
  }

  .table-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .table-row { border-bottom: 1px solid var(--theme-border-primary); }
  .table-row:last-child { border-bottom: none; }
  .table-row:hover { background-color: var(--theme-bg-hover); }

  .row-name { font-family: var(--font-display); font-size: 11px; color: var(--theme-text-primary); }
  .row-date { font-family: var(--font-mono); font-size: 10px; color: var(--theme-text-muted); }

  .edit-link { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-accent); text-decoration: none; }
  .edit-link:hover { text-decoration: underline; }

  .delete-btn {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 8px;
    background: none;
    border: 2px solid var(--color-text-error);
    color: var(--color-text-error);
    cursor: pointer;
  }
  .delete-btn:hover { background-color: var(--color-text-error); color: var(--theme-bg-primary); }
</style>
