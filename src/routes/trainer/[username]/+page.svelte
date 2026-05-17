<script lang="ts">
  let { data } = $props();

  let joinDate = $derived(
    data.trainer.createdAt
      ? new Date(data.trainer.createdAt).toLocaleDateString('pt-BR')
      : '—'
  );
</script>

<svelte:head>
  <title>{data.trainer.username} | Karta Yaxu</title>
</svelte:head>

<div class="trainer-page">

  <div class="profile-header">
    <div class="username-row">
      <h2>{data.trainer.username.toUpperCase()}</h2>
      <span class="role-badge" class:editor={data.trainer.role === 'editor'}>
        {data.trainer.role.toUpperCase()}
      </span>
      {#if data.isOwnProfile}
        <span class="you-badge">YOU</span>
      {/if}
    </div>
    <div class="join-date">Joined: {joinDate}</div>
  </div>

  <div class="stats-row">
    <div class="stat-block">
      <span class="stat-value">{data.stats.teamCount}</span>
      <span class="stat-label">TEAMS</span>
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.stats.battleCount}</span>
      <span class="stat-label">BATTLES</span>
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.stats.editCount}</span>
      <span class="stat-label">EDITS</span>
    </div>
  </div>

  <section class="profile-section">
    <h3 class="section-title">TEAMS</h3>
    <div class="empty-state">
      <p>No teams yet.</p>
      <p class="hint">Team builder coming in a future update.</p>
    </div>
  </section>

  <section class="profile-section">
    <h3 class="section-title">BATTLES</h3>
    <div class="empty-state">
      <p>No battles yet.</p>
      <p class="hint">Battle history will appear here.</p>
    </div>
  </section>

  {#if data.isOwnProfile || data.stats.editCount > 0}
    <section class="profile-section">
      <h3 class="section-title">EDIT HISTORY</h3>
      {#if data.stats.editCount === 0}
        <div class="empty-state">
          <p>No edits yet.</p>
          <p class="hint">Changes made in the Editor will be recorded here.</p>
        </div>
      {:else}
        <a href="/trainer/{data.trainer.username}/edits" class="edits-link card-interactive">
          <span>View {data.stats.editCount} edit{data.stats.editCount === 1 ? '' : 's'}</span>
          <span class="arrow">→</span>
        </a>
      {/if}
    </section>
  {/if}

</div>

<style>
  .trainer-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .profile-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-6);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .username-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    flex-wrap: wrap;
  }

  .username-row h2 {
    margin: 0;
    color: var(--theme-text-primary);
  }

  .role-badge {
    font-family: var(--font-display);
    font-size: 9px;
    padding: 2px 10px;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-muted);
    background-color: var(--theme-bg-primary);
  }

  .role-badge.editor {
    color: var(--theme-text-accent);
    border-color: var(--theme-text-accent);
  }

  .you-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 8px;
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .join-date {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .stats-row {
    display: flex;
    gap: var(--spacing-4);
  }

  .stat-block {
    flex: 1;
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-2);
    text-align: center;
  }

  .stat-value {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    line-height: 1;
  }

  .stat-label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-muted);
    letter-spacing: 1px;
  }

  .profile-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    margin: 0;
    padding-bottom: var(--spacing-2);
    border-bottom: 2px solid var(--theme-border-primary);
  }

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

  .edits-link {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-4);
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .arrow {
    color: var(--theme-text-accent);
  }
</style>
