<script lang="ts">
  import { enhance } from '$app/forms';

  let { data, form } = $props();

  let saved = $state(false);

  function handleSave() {
    saved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        saved = true;
        setTimeout(() => saved = false, 2500);
      }
    };
  }

  function formatDate(ts: Date | null): string {
    if (!ts) return '—';
    return new Date(ts).toLocaleDateString('pt-BR');
  }
</script>

<svelte:head>
  <title>Edit {data.trainer.username} | Karta Yaxu</title>
</svelte:head>

<div class="edit-page">
  <header class="page-header">
    <div class="header-left">
      <h2>EDIT NPC: {data.trainer.username.toUpperCase()}</h2>
      <span class="npc-badge">NPC</span>
    </div>
    <div class="meta">
      <span>ID: {data.trainer.id}</span>
      <span>CREATED: {formatDate(data.trainer.createdAt)}</span>
    </div>
  </header>

  <form action="?/update" method="post" use:enhance={handleSave} class="edit-form">
    <div class="field">
      <label for="username">USERNAME</label>
      <input id="username" name="username" type="text" maxlength="30" required
        value={form?.values?.username ?? data.trainer.username}>
    </div>
    <div class="form-actions">
      <button type="submit" class="save-btn">SAVE</button>
      {#if form?.updateError}
        <span class="form-error">{form.updateError}</span>
      {/if}
      {#if saved}
        <span class="save-ok">✓ SAVED</span>
      {/if}
    </div>
  </form>
</div>

<style>
  .edit-page { display: flex; flex-direction: column; gap: var(--spacing-5); }

  .page-header {
    display: flex; justify-content: space-between; align-items: flex-end;
    gap: var(--spacing-3); flex-wrap: wrap;
  }
  .header-left { display: flex; align-items: center; gap: var(--spacing-3); flex-wrap: wrap; }
  .page-header h2 { color: var(--theme-text-primary); margin: 0; }
  .meta { display: flex; gap: var(--spacing-3); font-family: var(--font-mono); font-size: 11px; color: var(--theme-text-muted); }
  .wiki-link { color: var(--theme-text-accent); text-decoration: none; }
  .wiki-link:hover { text-decoration: underline; }

  .npc-badge {
    font-family: var(--font-display); font-size: 8px; padding: 2px 6px;
    border: 2px solid var(--color-text-warning); color: var(--color-text-warning);
  }

  .edit-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .field { display: flex; flex-direction: column; gap: var(--spacing-1); }
  .field label { font-family: var(--font-display); font-size: 9px; color: var(--theme-text-muted); }
  .field input {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }
  .field input:focus { outline: none; border-color: var(--theme-text-accent); }

  .form-actions { grid-column: 1 / -1; display: flex; align-items: center; gap: var(--spacing-3); }
  .save-btn {
    font-family: var(--font-display); font-size: 11px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none; border: 2px solid var(--theme-text-accent); color: var(--theme-text-accent);
    cursor: pointer;
  }
  .save-btn:hover { background-color: var(--theme-text-accent); color: var(--theme-bg-primary); }
  .form-error { font-family: var(--font-mono); font-size: 11px; color: var(--color-text-error); }
  .save-ok { font-family: var(--font-display); font-size: 10px; color: var(--theme-text-accent); }
</style>
