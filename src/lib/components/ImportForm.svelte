<script lang="ts">
  import { enhance } from '$app/forms';
  import { startImport } from '$lib/stores/importActivity';

  interface SelectOption {
    value: string | number;
    label: string;
  }

  interface Field {
    name: string;
    type: 'number' | 'text' | 'select';
    placeholder?: string;
    label?: string;
    min?: number;
    options?: SelectOption[];
  }

  let {
    action,
    fields = [] as Field[],
    submitLabel = 'IMPORT',
  } = $props();

  let state = $state<'idle' | 'loading' | 'success' | 'error'>('idle');
  let message = $state('');

  function handleSubmit() {
    state = 'loading';
    message = '';
    startImport();
    return async ({ result, update }: any) => {
      if (result.type === 'success') {
        state = 'success';
        message = result.data?.message || 'Done.';
      } else {
        state = 'error';
        message = result.data?.error || 'Failed.';
      }
      await update();
      if (state === 'success') {
        setTimeout(() => { state = 'idle'; message = ''; }, 3000);
      }
    };
  }
</script>

<div class="import-form">
  <form method="POST" {action} use:enhance={handleSubmit}>
    <div class="form-row">
      {#each fields as f}
        <div class="field">
          {#if f.label}<label for={f.name}>{f.label}</label>{/if}
          {#if f.type === 'select'}
            <select id={f.name} name={f.name} class="select-input" required>
              {#each f.options ?? [] as opt}
                <option value={opt.value}>{opt.label}</option>
              {/each}
            </select>
          {:else}
            <input
              id={f.name}
              type={f.type}
              name={f.name}
              placeholder={f.placeholder}
              min={f.min}
              required
            />
          {/if}
        </div>
      {/each}

      <button
        type="submit"
        class="btn"
        class:loading={state === 'loading'}
        class:success={state === 'success'}
        class:error={state === 'error'}
        disabled={state === 'loading'}
      >
        {#if state === 'loading'}
          <span class="spinner"></span> IMPORTING...
        {:else if state === 'success'}
          ✅ DONE
        {:else if state === 'error'}
          ❌ ERROR
        {:else}
          {submitLabel}
        {/if}
      </button>
    </div>
  </form>

  {#if message}
    <p class="message" class:error-msg={state === 'error'}>{message}</p>
  {/if}
</div>

<style>
  .import-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .form-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }

  .field {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  label {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    white-space: nowrap;
  }

  input {
    width: 70px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
  }

  input[type="text"] { width: 120px; }

  .select-input {
    background-color: var(--theme-bg-primary);
    color: var(--theme-text-primary);
    border: 2px solid var(--theme-border-primary);
    padding: var(--spacing-2) var(--spacing-3);
    font-family: var(--font-display);
    font-size: 10px;
    min-width: 140px;
  }

  .btn {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    padding: var(--spacing-2) var(--spacing-4);
    background-color: var(--theme-bg-secondary);
    border: 2px solid var(--theme-border-primary);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.1s ease;
  }

  .btn:not(:disabled):hover {
    background-color: var(--theme-bg-hover);
    transform: translate(-1px, -1px);
    box-shadow: 3px 3px 0px var(--theme-border-primary);
  }

  .btn:disabled { opacity: 0.7; cursor: not-allowed; }
  .btn.success { border-color: var(--theme-text-primary); }
  .btn.error { background-color: var(--color-text-error); color: var(--color-bg-primary); }

  .spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 2px solid var(--theme-text-secondary);
    border-top-color: var(--theme-text-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .message {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
    margin: 0;
  }

  .message.error-msg { color: var(--color-text-error); }
</style>
