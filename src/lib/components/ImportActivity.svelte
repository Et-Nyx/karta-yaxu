<script lang="ts">
  import { page } from '$app/stores';
  import { importActivity, stagingActivity } from '$lib/stores/importActivity';
  import RetroInput from './RetroInput.svelte';
  import RetroButton from './RetroButton.svelte';

  let stagingComment = $state('');
  let isMigrating = $state(false);
  let migrationError = $state('');

  async function applyStagedMigration() {
    if (!stagingComment.trim()) return;
    isMigrating = true;
    migrationError = '';
    try {
      const res = await fetch('/api/editor/commit-import', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ comment: stagingComment }),
      });
      const resData = await res.json();
      if (resData.success) {
        stagingActivity.update((s) => ({ ...s, status: 'idle' }));
        stagingComment = '';
        window.location.reload();
      } else {
        migrationError = resData.error || 'Failed to migrate staged payloads.';
      }
    } catch (e: any) {
      migrationError = e.message || 'An error occurred.';
    } finally {
      isMigrating = false;
    }
  }

  async function clearStagedStaging() {
    try {
      await fetch('/api/editor/clear-staging', { method: 'POST' });
      stagingActivity.update((s) => ({ ...s, status: 'idle' }));
      stagingComment = '';
    } catch {}
  }
</script>

{#if $stagingActivity.status !== 'idle' && $page.data.user && $page.data.user.role === 'editor'}
  <!-- Combined Background Staging Activity -->
  <div class="activity active staging-box">
    <div class="activity-header">
      {#if $stagingActivity.status === 'fetching'}
        <span class="dot"></span>
        <span class="status-label">STAGING ACTIVE</span>
      {:else if $stagingActivity.status === 'migrating'}
        <span class="dot pulse"></span>
        <span class="status-label status-migrating">MIGRATING DB</span>
      {:else if $stagingActivity.status === 'fetched'}
        <span class="done-mark">✓</span>
        <span class="status-label status-ready">STAGING READY</span>
      {:else if $stagingActivity.status === 'failed'}
        <span class="done-mark" style="color: var(--color-text-error);">⚠</span>
        <span class="status-label" style="color: var(--color-text-error);">STAGING FAILED</span>
      {/if}
    </div>

    {#if $stagingActivity.status === 'fetching'}
      <div class="entity-type">
        STAGING {$stagingActivity.entityType ? $stagingActivity.entityType.toUpperCase() : '...'}
      </div>
      <div class="progress-bar-container">
        <div
          class="progress-bar"
          style="width: {Math.min(
            100,
            Math.round(($stagingActivity.fetched / $stagingActivity.total) * 100)
          )}%"
        ></div>
        <span class="progress-text"
          >{Math.min(100, Math.round(($stagingActivity.fetched / $stagingActivity.total) * 100))}%</span
        >
      </div>
      <div class="staging-counts">
        Staged {$stagingActivity.fetched} / {$stagingActivity.total} payloads
      </div>
      <div class="bg-reminder">
        Background active. You can browse the wiki freely.
      </div>
    {:else if $stagingActivity.status === 'migrating'}
      <div class="entity-type">
        APPLYING TRANSACTIONS...
      </div>
      <div class="progress-bar-container">
        <div
          class="progress-bar"
          style="width: {Math.min(
            100,
            Math.round(($stagingActivity.fetched / $stagingActivity.total) * 100)
          )}%"
        ></div>
        <span class="progress-text"
          >{Math.min(100, Math.round(($stagingActivity.fetched / $stagingActivity.total) * 100))}%</span
        >
      </div>
      {#if $stagingActivity.activeName}
        <div class="active-item-info">
          Migrating: #{ $stagingActivity.activeId } { $stagingActivity.activeName }
        </div>
      {/if}
      <div class="staging-counts">
        Migrated {$stagingActivity.fetched} / {$stagingActivity.total} species
      </div>
      <div class="bg-reminder" style="color: var(--color-text-warning);">
        Writing atomically. Do not close the window.
      </div>
    {:else if $stagingActivity.status === 'fetched'}
      <div class="entity-type">
        READY: {$stagingActivity.total} {$stagingActivity.entityType
          ? $stagingActivity.entityType.toUpperCase()
          : ''}
      </div>

      <div class="comment-field">
        <span class="field-title">COMMIT / AUDIT NOTE:</span>
        <RetroInput
          type="text"
          placeholder="e.g. Seed Kanto species"
          bind:value={stagingComment}
          disabled={isMigrating}
          class="sidebar-retro-input"
        />
      </div>

      {#if migrationError}
        <div class="error-msg">⚠ {migrationError}</div>
      {/if}

      <div class="action-buttons">
        <RetroButton
          onclick={applyStagedMigration}
          variant="primary"
          disabled={isMigrating || !stagingComment.trim()}
        >
          {isMigrating ? 'MIGRATING...' : 'APPLY'}
        </RetroButton>
        <RetroButton onclick={clearStagedStaging} variant="secondary" disabled={isMigrating}>
          CANCEL
        </RetroButton>
      </div>
    {:else if $stagingActivity.status === 'failed'}
      <div class="error-msg">Staging process failed. Please try again.</div>
      <RetroButton onclick={clearStagedStaging} variant="secondary">RESET</RetroButton>
    {/if}
  </div>
{:else if $importActivity.isActive || $importActivity.entityType !== null}
  <!-- Standard Import Log Status Block -->
  <div class="activity" class:active={$importActivity.isActive}>
    <div class="activity-header">
      {#if $importActivity.isActive}
        <span class="dot"></span>
        <span class="status-label">IMPORTING</span>
      {:else}
        <span class="done-mark">✓</span>
        <span class="status-label">DONE</span>
      {/if}
    </div>

    <div class="entity-type">
      {$importActivity.entityType ? $importActivity.entityType.toUpperCase() : '...'}
    </div>

    {#if $importActivity.entityType}
      <div class="counts">
        <span class="count ins">{$importActivity.inserted} ins</span>
        {#if $importActivity.updated > 0}
          <span class="count upd">{$importActivity.updated} upd</span>
        {/if}
        {#if $importActivity.skipped > 0}
          <span class="count skp">{$importActivity.skipped} skp</span>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .activity {
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    background-color: var(--theme-bg-secondary);
    margin-bottom: var(--spacing-4);
    opacity: 0.9;
    transition: opacity 0.2s ease;
    font-family: var(--font-mono);
  }

  .activity.active {
    opacity: 1;
    border-color: var(--theme-text-primary);
  }

  .staging-box {
    border: 4px double var(--theme-border-primary);
    box-shadow: 3px 3px 0px rgba(0, 0, 0, 0.2);
  }

  .activity-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    margin-bottom: var(--spacing-2);
  }

  .status-label {
    font-family: var(--font-display);
    font-size: 8px;
    letter-spacing: 0.5px;
    color: var(--theme-text-muted);
  }

  .active .status-label {
    color: var(--theme-text-primary);
  }

  .status-ready {
    color: var(--theme-text-accent) !important;
  }

  .status-migrating {
    color: var(--theme-text-accent) !important;
  }

  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    background-color: var(--theme-text-primary);
    border-radius: 50%;
    animation: pulse 1s ease-in-out infinite;
  }

  .done-mark {
    font-size: 10px;
    color: var(--theme-text-accent);
    font-weight: bold;
  }

  .entity-type {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
  }

  .progress-bar-container {
    width: 100%;
    height: 14px;
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    position: relative;
    overflow: hidden;
    margin-bottom: var(--spacing-2);
  }

  .progress-bar {
    height: 100%;
    background-color: var(--theme-text-accent);
    transition: width 0.3s ease;
  }

  .progress-text {
    position: absolute;
    width: 100%;
    text-align: center;
    top: 50%;
    transform: translateY(-50%);
    font-size: 8px;
    font-weight: bold;
    color: var(--theme-text-primary);
    mix-blend-mode: difference;
  }

  .staging-counts {
    font-size: 8px;
    color: var(--theme-text-muted);
    margin-bottom: var(--spacing-1);
  }

  .active-item-info {
    font-size: 8px;
    font-weight: bold;
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bg-reminder {
    font-size: 7px;
    color: var(--theme-text-muted);
    line-height: 1.3;
  }

  .comment-field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
    margin-bottom: var(--spacing-2);
  }

  .field-title {
    font-size: 7px;
    font-family: var(--font-display);
    color: var(--theme-text-secondary);
  }

  :global(.sidebar-retro-input) {
    width: 100%;
    font-size: 8px !important;
    padding: 3px var(--spacing-1) !important;
  }

  .action-buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-2);
    width: 100%;
  }

  .action-buttons :global(button) {
    font-size: 8px !important;
    padding: var(--spacing-2) var(--spacing-1) !important;
    width: 100% !important;
    text-align: center;
  }

  .error-msg {
    font-size: 8px;
    color: var(--color-text-error, #ff5555);
    margin-bottom: var(--spacing-2);
  }

  .counts {
    display: flex;
    gap: var(--spacing-2);
    flex-wrap: wrap;
  }

  .count {
    font-size: 8px;
    padding: 1px var(--spacing-1);
  }

  .ins {
    color: var(--theme-text-primary);
  }
  .upd {
    color: var(--color-text-warning, #d4a017);
  }
  .skp {
    color: var(--theme-text-muted);
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }
</style>
