<script lang="ts">
  // Redirect fallback
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  onMount(() => {
    goto('/editor');
  });
</script>

<div class="redirect-state">
  <p>Redirecting to Database Editor...</p>
</div>

<style>
  .redirect-state {
    font-family: var(--font-mono);
    color: var(--theme-text-muted);
    padding: var(--spacing-6);
    text-align: center;
  }
</style>
