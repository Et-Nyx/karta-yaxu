<script lang="ts">
  import { enhance } from '$app/forms';
  import { startImport } from '$lib/stores/importActivity';
  import RetroButton from './RetroButton.svelte';
  import RetroSelect from './RetroSelect.svelte';
  import RetroInput from './RetroInput.svelte';

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
    showOverwriteOption = false,
  } = $props();

  let btnState: 'idle' | 'loading' | 'success' | 'error' = $state('idle');
  let message = $state('');

  // Keep track of values for RetroSelect elements
  let selectValues = $state<Record<string, any>>({});

  // Initialize selected values
  $effect(() => {
    for (const f of fields) {
      if (f.type === 'select' && selectValues[f.name] === undefined) {
        selectValues[f.name] = f.options?.[0]?.value ?? '';
      }
    }
  });

  function handleSubmit() {
    btnState = 'loading';
    message = '';
    startImport();
    return async ({ result, update }: any) => {
      if (result.type === 'success') {
        btnState = 'success';
        message = result.data?.message || 'Done.';
      } else {
        btnState = 'error';
        message = result.data?.error || 'Failed.';
      }
      await update();
      if (btnState === 'success') {
        setTimeout(() => { btnState = 'idle'; message = ''; }, 3000);
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
            <RetroSelect
              id={f.name}
              name={f.name}
              bind:value={selectValues[f.name]}
              options={f.options ?? []}
              variant="editor"
              class="select-input"
            />
          {:else}
            <RetroInput
              id={f.name}
              type={f.type}
              name={f.name}
              placeholder={f.placeholder}
              min={f.min}
              class="form-retro-input"
              required
            />
          {/if}
        </div>
      {/each}

      {#if showOverwriteOption}
        <div class="field checkbox-field">
          <input
            id="overwriteEdited"
            type="checkbox"
            name="overwriteEdited"
            value="true"
            class="checkbox-input"
          />
          <label for="overwriteEdited" class="checkbox-label">OVERWRITE EDITED</label>
        </div>
      {/if}


      <RetroButton
        type="submit"
        disabled={btnState === 'loading'}
        class="{btnState === 'loading' ? 'loading' : ''} {btnState === 'success' ? 'success' : ''} {btnState === 'error' ? 'error' : ''}"
      >
        {#if btnState === 'loading'}
          <span class="spinner"></span> LOADING
        {:else if btnState === 'success'}
          ✓ SUCCESS
        {:else if btnState === 'error'}
          ✗ ERROR
        {:else}
          {submitLabel}
        {/if}
      </RetroButton>
    </div>
  </form>

  {#if message}
    <div class="form-message" class:success={btnState === 'success'} class:error={btnState === 'error'}>
      {message}
    </div>
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
    gap: var(--spacing-4);
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

  :global(.import-form .form-retro-input) {
    width: 80px;
    height: 32px;
    padding: var(--spacing-2) var(--spacing-3) !important;
  }

  :global(.import-form .form-retro-input[type="text"]) {
    width: 140px;
  }


  .checkbox-field {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    cursor: pointer;
    user-select: none;
  }

  .checkbox-input {
    width: 18px;
    height: 18px;
    cursor: pointer;
    accent-color: var(--theme-text-accent);
    border: 2px solid var(--theme-border-primary);
    border-radius: 0;
    appearance: none;
    background-color: var(--theme-bg-secondary);
    box-shadow: 2px 2px 0 var(--theme-border-primary);
    position: relative;
  }

  .checkbox-input:checked::before {
    content: "✓";
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
  }

  .checkbox-label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-primary);
    cursor: pointer;
  }

  .form-message {
    font-family: var(--font-mono);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    border: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    color: var(--theme-text-primary);
  }

  .form-message.success {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .form-message.error {
    border-color: var(--color-text-error);
    color: var(--color-text-error);
  }

  .spinner {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-right: var(--spacing-2);
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
