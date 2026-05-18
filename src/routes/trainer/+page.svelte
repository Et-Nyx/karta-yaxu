<script lang="ts">
  import { enhance } from '$app/forms';
  import DataTable from '$lib/components/DataTable.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import Tabs from '$lib/components/Tabs.svelte';

  let { data } = $props();

  let query = $state('');
  let activeTab = $state('users');

  let filteredUsers = $derived(
    data.users.filter(u => u.username.toLowerCase().includes(query.toLowerCase()))
  );
  let filteredNpcs = $derived(
    data.npcs.filter(n => n.username.toLowerCase().includes(query.toLowerCase()))
  );

  const tabs = [
    { key: 'users', label: 'USERS' },
    { key: 'npcs',  label: 'NPCs' },
  ];

  let currentCount = $derived(activeTab === 'users' ? filteredUsers.length : filteredNpcs.length);
  let currentTotal = $derived(activeTab === 'users' ? data.users.length : data.npcs.length);

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

  <SearchBar bind:value={query} placeholder="Search trainers..." count={currentCount} total={currentTotal} />

  <Tabs {tabs} bind:active={activeTab}>
    {#if activeTab === 'users'}
      {#if filteredUsers.length === 0}
        <div class="empty-state"><p>No users found.</p></div>
      {:else}
        <DataTable>
          {#snippet head()}
            <th class="col-num">#</th>
            <th>USERNAME</th>
            <th class="center">ROLE</th>
            <th class="center">JOINED</th>
            <th class="center">STATUS</th>
            {#if data.isEditor}<th>ACTIONS</th>{/if}
          {/snippet}
          {#each filteredUsers as trainer, i (trainer.id)}
            {@const isSelf = trainer.id === data.currentUserId}
            {@const isBanned = !!trainer.bannedAt}
            <tr class:banned={isBanned}>
              <td class="col-num muted">{i + 1}</td>
              <td class="name-cell"><a href="/trainer/{trainer.username}">{trainer.username}</a></td>
              <td class="center">
                <span class="role-badge" class:editor={trainer.role === 'editor'}>
                  {trainer.role.toUpperCase()}
                </span>
              </td>
              <td class="center muted small">{formatDate(trainer.createdAt)}</td>
              <td class="center small" class:status-banned={isBanned}>
                {isBanned ? 'BANNED' : 'ACTIVE'}
              </td>
              {#if data.isEditor}
                <td class="actions-cell">
                  {#if !isSelf}
                    <div class="action-forms">
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
                    </div>
                  {:else}
                    <span class="self-label">YOU</span>
                  {/if}
                </td>
              {/if}
            </tr>
          {/each}
        </DataTable>
      {/if}

    {:else}
      {#if filteredNpcs.length === 0}
        <div class="empty-state"><p>No NPCs found.</p></div>
      {:else}
        <DataTable>
          {#snippet head()}
            <th>NAME</th>
          {/snippet}
          {#each filteredNpcs as npc (npc.id)}
            <tr>
              <td class="name-cell"><a href="/trainer/{npc.username}">{npc.username}</a></td>
            </tr>
          {/each}
        </DataTable>
      {/if}
    {/if}
  </Tabs>
</div>

<style>
  .trainers-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header {
    display: flex;
    align-items: baseline;
    gap: var(--spacing-4);
  }

  .page-header h2 { margin: 0; color: var(--theme-text-primary); }

  .total {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  /* Table cell helpers */
  .col-num { width: 36px; }
  .center { text-align: center; }

  :global(tr.banned) { opacity: 0.55; }

  .muted { color: var(--theme-text-muted); }

  .small { font-family: var(--font-display); font-size: 9px; }

  .name-cell { font-family: var(--font-display); font-size: 11px; }
  .name-cell a { color: var(--theme-text-primary); text-decoration: none; }
  .name-cell a:hover { color: var(--theme-text-accent); }

  .status-banned { color: var(--color-text-error) !important; }

  .action-forms { display: flex; gap: var(--spacing-2); align-items: center; }

  .role-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 8px;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-primary);
    white-space: nowrap;
  }
  .role-badge.editor { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }

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
  .action-btn:hover { background-color: var(--theme-bg-hover); }
  .action-btn.promote { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .action-btn.demote { color: var(--theme-text-muted); }
  .action-btn.ban { color: var(--color-text-error); border-color: var(--color-text-error); }
  .action-btn.unban { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }

  .self-label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-muted);
    padding: 2px 8px;
  }

  .empty-state {
    text-align: center;
    padding: var(--spacing-5);
  }
  .empty-state p {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    margin: 0;
  }
</style>
