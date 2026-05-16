<script lang="ts">
  import { enhance } from '$app/forms';

  let { data, form } = $props();

  let npcName = $state('');

  function formatDate(ts: Date | null): string {
    if (!ts) return '—';
    return new Date(ts).toLocaleDateString('pt-BR');
  }
</script>

<svelte:head>
  <title>Trainers | Karta Yaxu</title>
</svelte:head>

<div class="trainers-page">
  <div class="page-header">
    <h2>TRAINERS</h2>
    <span class="total">{data.users.length + data.npcs.length} TOTAL</span>
  </div>

  <!-- USERS -->
  <section class="section">
    <h3 class="section-title">USERS ({data.users.length})</h3>

    {#if data.users.length === 0}
      <div class="empty-state"><p>No users registered yet.</p></div>
    {:else}
      <div class="trainer-table">
        <div class="table-header">
          <span class="col-num">#</span>
          <span class="col-name">USERNAME</span>
          <span class="col-role">ROLE</span>
          <span class="col-date">JOINED</span>
          <span class="col-status">STATUS</span>
          {#if data.isEditor}<span class="col-actions">ACTIONS</span>{/if}
        </div>

        {#each data.users as trainer, i (trainer.id)}
          {@const isSelf = trainer.id === data.currentUserId}
          {@const isBanned = !!trainer.bannedAt}
          <div class="table-row" class:banned={isBanned}>
            <span class="col-num row-num">{i + 1}</span>
            <span class="col-name row-name">
              <a href="/trainer/{trainer.username}">{trainer.username}</a>
            </span>
            <span class="col-role">
              <span class="role-badge" class:editor={trainer.role === 'editor'}>
                {trainer.role.toUpperCase()}
              </span>
            </span>
            <span class="col-date row-date">{formatDate(trainer.createdAt)}</span>
            <span class="col-status row-status" class:status-banned={isBanned}>
              {isBanned ? 'BANNED' : 'ACTIVE'}
            </span>
            {#if data.isEditor}
              <span class="col-actions row-actions">
                {#if !isSelf}
                  {#if !isBanned}
                    {#if trainer.role === 'viewer'}
                      <form action="?/promote" method="post" use:enhance>
                        <input type="hidden" name="id" value={trainer.id}>
                        <button type="submit" class="action-btn promote">PROMOTE</button>
                      </form>
                    {:else}
                      <form action="?/demote" method="post" use:enhance>
                        <input type="hidden" name="id" value={trainer.id}>
                        <button type="submit" class="action-btn demote">DEMOTE</button>
                      </form>
                    {/if}
                    <form action="?/ban" method="post" use:enhance>
                      <input type="hidden" name="id" value={trainer.id}>
                      <button type="submit" class="action-btn ban">BAN</button>
                    </form>
                  {:else}
                    <form action="?/unban" method="post" use:enhance>
                      <input type="hidden" name="id" value={trainer.id}>
                      <button type="submit" class="action-btn unban">UNBAN</button>
                    </form>
                  {/if}
                {:else}
                  <span class="self-label">YOU</span>
                {/if}
              </span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- NPCs -->
  <section class="section">
    <h3 class="section-title">NPCs ({data.npcs.length})</h3>

    {#if data.isEditor}
      <form
        action="?/createNpc"
        method="post"
        class="create-npc-form"
        use:enhance={({ formElement }) => {
          return async ({ result, update }) => {
            await update();
            if (result.type === 'success') {
              npcName = '';
              formElement.reset();
            }
          };
        }}
      >
        <input
          type="text"
          name="username"
          bind:value={npcName}
          placeholder="NPC NAME"
          class="npc-input"
          maxlength="30"
          required
        >
        <button type="submit" class="create-btn">+ ADD NPC</button>
      </form>
      {#if form?.createError}
        <p class="form-error">{form.createError}</p>
      {/if}
    {/if}

    {#if data.npcs.length === 0}
      <div class="empty-state">
        <p>No NPCs yet.</p>
        {#if data.isEditor}
          <p class="hint">Add game characters using the form above.</p>
        {/if}
      </div>
    {:else}
      <div class="npc-table">
        <div class="table-header">
          <span class="col-name">NAME</span>
          {#if data.isEditor}<span class="col-actions">ACTIONS</span>{/if}
        </div>
        {#each data.npcs as npc (npc.id)}
          <div class="table-row">
            <span class="col-name row-name">
              <a href="/trainer/{npc.username}">{npc.username}</a>
            </span>
            {#if data.isEditor}
              <span class="col-actions row-actions">
                <form action="?/deleteNpc" method="post" use:enhance>
                  <input type="hidden" name="id" value={npc.id}>
                  <button type="submit" class="action-btn ban">DELETE</button>
                </form>
              </span>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .trainers-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header {
    display: flex;
    align-items: baseline;
    gap: var(--spacing-4);
  }

  .page-header h2 {
    margin: 0;
    color: var(--theme-text-primary);
  }

  .total {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

  /* Tables */
  .trainer-table,
  .npc-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
    overflow: hidden;
  }

  .table-header,
  .table-row {
    display: grid;
    align-items: center;
    padding: var(--spacing-2) var(--spacing-4);
    gap: var(--spacing-3);
  }

  .trainer-table .table-header,
  .trainer-table .table-row {
    grid-template-columns: 32px 1fr 80px 100px 70px auto;
  }

  .npc-table .table-header,
  .npc-table .table-row {
    grid-template-columns: 1fr auto;
  }

  .table-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .table-row {
    border-bottom: 1px solid var(--theme-border-primary);
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background-color: var(--theme-bg-hover);
  }

  .table-row.banned {
    opacity: 0.55;
  }

  /* Columns */
  .row-num {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .row-name {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .row-name a {
    color: var(--theme-text-primary);
  }

  .row-name a:hover {
    color: var(--theme-text-accent);
  }

  .row-date {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .row-status {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .row-status.status-banned {
    color: var(--color-text-error);
  }

  .row-actions {
    display: flex;
    gap: var(--spacing-2);
    align-items: center;
    justify-content: flex-end;
  }

  /* Role badge */
  .role-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 8px;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-primary);
    white-space: nowrap;
  }

  .role-badge.editor {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  /* Action buttons */
  .action-btn {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 8px;
    border: 2px solid var(--theme-border-primary);
    background: none;
    cursor: pointer;
    color: var(--theme-text-primary);
    white-space: nowrap;
  }

  .action-btn:hover {
    background-color: var(--theme-bg-hover);
  }

  .action-btn.promote {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .action-btn.demote {
    color: var(--theme-text-muted);
  }

  .action-btn.ban {
    color: var(--color-text-error);
    border-color: var(--color-text-error);
  }

  .action-btn.unban {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .self-label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-muted);
    padding: 2px 8px;
  }

  /* NPC create form */
  .create-npc-form {
    display: flex;
    gap: var(--spacing-2);
    align-items: center;
  }

  .npc-input {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-2) var(--spacing-3);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    width: 220px;
  }

  .npc-input:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .create-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    border: 2px solid var(--theme-border-primary);
    background: none;
    cursor: pointer;
    color: var(--theme-text-primary);
    white-space: nowrap;
  }

  .create-btn:hover {
    background-color: var(--theme-bg-hover);
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
    margin: 0;
  }

  /* Empty state */
  .empty-state {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-5);
    text-align: center;
  }

  .empty-state p {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-1) 0;
  }

  .empty-state .hint {
    font-size: 11px;
    color: var(--theme-text-muted);
  }
</style>
