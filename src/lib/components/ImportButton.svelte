<script lang="ts">
  import { enhance } from '$app/forms';
  import { startImport } from '$lib/stores/importActivity';
  import RetroButton from './RetroButton.svelte';

  let {
    action,
    label,
    loadingLabel = 'IMPORTING...',
    disabled = false
  } = $props();

  let btnState: 'idle' | 'loading' | 'success' | 'error' = $state('idle');
  let resultMessage = $state('');

  function handleSubmit() {
    btnState = 'loading';
    resultMessage = '';
    startImport();

    return async ({ result, update }: any) => {
      if (result.type === 'success') {
        btnState = 'success';
        resultMessage = result.data?.message || 'Success!';
      } else {
        btnState = 'error';
        resultMessage = result.data?.error || 'Failed.';
      }
      await update();

      if (btnState === 'success') {
        setTimeout(() => {
          btnState = 'idle';
          resultMessage = '';
        }, 3000);
      }
    };
  }
</script>

<div class="import-button-container">
  <form method="POST" {action} use:enhance={handleSubmit}>
    <RetroButton 
      type="submit" 
      disabled={disabled || btnState === 'loading'}
      class="{btnState === 'loading' ? 'loading' : ''} {btnState === 'success' ? 'success' : ''} {btnState === 'error' ? 'error' : ''}"
    >
      {#if btnState === 'loading'}
        <span class="spinner"></span> {loadingLabel}
      {:else if btnState === 'success'}
        ✅ DONE
      {:else if btnState === 'error'}
        ❌ ERROR
      {:else}
        {label}
      {/if}
    </RetroButton>
  </form>
  
  {#if resultMessage}
    <div class="result-message" class:error-msg={btnState === 'error'}>
      {resultMessage}
    </div>
  {/if}
</div>

<style>
  .import-button-container {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--spacing-2);
  }

  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--theme-text-secondary);
    border-top-color: var(--theme-text-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .result-message {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin-top: 4px;
    margin-left: 2px;
  }

  .result-message.error-msg {
    color: var(--color-text-error);
  }
</style>
