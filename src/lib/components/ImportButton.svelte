<script lang="ts">
  import { enhance } from '$app/forms';
  import { startImport } from '$lib/stores/importActivity';

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
    <button 
      type="submit" 
      class="btn" 
      class:loading={btnState === 'loading'}
      class:success={btnState === 'success'}
      class:error={btnState === 'error'}
      disabled={disabled || btnState === 'loading'}
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
    </button>
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

  .btn {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    padding: var(--spacing-3) var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card-subtle);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .btn:not(:disabled):hover {
    background-color: var(--theme-bg-hover);
    transform: translate(-2px, -2px);
    box-shadow: var(--shadow-card-hover);
  }

  .btn:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }

  .btn.success {
    background-color: var(--theme-bg-hover);
    border-color: var(--theme-text-primary);
  }

  .btn.error {
    background-color: var(--color-text-error);
    color: var(--color-bg-primary);
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
    margin-left: 2px;
  }

  .result-message.error-msg {
    color: var(--color-text-error);
  }
</style>
