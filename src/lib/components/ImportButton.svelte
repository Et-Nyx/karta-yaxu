<script lang="ts">
  import { enhance } from '$app/forms';

  let { 
    action, 
    label, 
    loadingLabel = 'IMPORTING...', 
    disabled = false 
  } = $props();

  let state = $state<'idle' | 'loading' | 'success' | 'error'>('idle');
  let resultMessage = $state('');

  function handleSubmit() {
    state = 'loading';
    resultMessage = '';
    
    return async ({ result, update }: any) => {
      if (result.type === 'success') {
        state = 'success';
        resultMessage = result.data?.message || 'Success!';
      } else {
        state = 'error';
        resultMessage = result.data?.error || 'Failed.';
      }
      await update();
      
      // Reset after 3 seconds if success
      if (state === 'success') {
        setTimeout(() => {
          state = 'idle';
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
      class:loading={state === 'loading'}
      class:success={state === 'success'}
      class:error={state === 'error'}
      disabled={disabled || state === 'loading'}
    >
      {#if state === 'loading'}
        <span class="spinner"></span> {loadingLabel}
      {:else if state === 'success'}
        ✅ DONE
      {:else if state === 'error'}
        ❌ ERROR
      {:else}
        {label}
      {/if}
    </button>
  </form>
  
  {#if resultMessage}
    <div class="result-message" class:error-msg={state === 'error'}>
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
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .btn:not(:disabled):hover {
    background-color: var(--theme-bg-hover);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0px var(--theme-border-primary);
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
