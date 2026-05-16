<script lang="ts">
  import { enhance } from '$app/forms';

  let { form } = $props();
</script>

<svelte:head>
  <title>Login | Karta Yaxu</title>
</svelte:head>

<div class="auth-page">
  <div class="auth-card">
    <h2>LOGIN</h2>

    {#if form?.error}
      <div class="error-banner">{form.error}</div>
    {/if}

    <form method="post" use:enhance>
      <div class="field">
        <label for="username">USERNAME</label>
        <input id="username" name="username" type="text" autocomplete="username" required />
      </div>

      <div class="field">
        <label for="password">PASSWORD</label>
        <input id="password" name="password" type="password" autocomplete="current-password" required />
      </div>

      <button type="submit" class="submit-btn">ENTER</button>
    </form>

    <p class="alt-link">
      Sem conta? <a href="/trainer/register">REGISTRAR</a>
    </p>
  </div>
</div>

<style>
  .auth-page {
    display: flex;
    justify-content: center;
    padding: var(--spacing-8) 0;
  }

  .auth-card {
    width: 100%;
    max-width: 380px;
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-6);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-5);
  }

  h2 {
    font-family: var(--font-display);
    font-size: 18px;
    color: var(--theme-text-primary);
    margin: 0;
    text-align: center;
  }

  .error-banner {
    background-color: var(--theme-bg-primary);
    border: 2px solid #e53e3e;
    padding: var(--spacing-3);
    font-family: var(--font-mono);
    font-size: 11px;
    color: #e53e3e;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    letter-spacing: 1px;
  }

  input {
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: var(--spacing-2) var(--spacing-3);
    width: 100%;
    box-sizing: border-box;
  }

  input:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .submit-btn {
    margin-top: var(--spacing-2);
    padding: var(--spacing-3);
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    color: var(--theme-text-primary);
    font-family: var(--font-display);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .submit-btn:hover {
    transform: translate(-2px, -2px);
    box-shadow: var(--shadow-card-hover);
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .alt-link {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
    text-align: center;
    margin: 0;
  }

  .alt-link a {
    color: var(--theme-text-accent);
    text-decoration: none;
    font-family: var(--font-display);
    font-size: 10px;
  }

  .alt-link a:hover {
    text-decoration: underline;
  }
</style>
