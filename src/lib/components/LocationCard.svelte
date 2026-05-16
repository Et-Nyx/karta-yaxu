<script lang="ts">
  let {
    name,
    slug,
  }: {
    name: string;
    slug: string;
  } = $props();

  let displayName = $derived(
    name.replace(/-/g, ' ').replace(/\b\w/g, c => c.toUpperCase())
  );
</script>

<a href="/wiki/locations/{slug}" class="location-card card-interactive">
  {displayName}
</a>

<style>
  .location-card {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-primary);
    padding: var(--spacing-2) var(--spacing-4);
    white-space: nowrap;
  }
</style>
