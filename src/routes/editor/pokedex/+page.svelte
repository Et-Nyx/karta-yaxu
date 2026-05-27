<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import DataTable from '$lib/components/DataTable.svelte';
  import RetroInput from '$lib/components/RetroInput.svelte';
  import RetroSelect from '$lib/components/RetroSelect.svelte';
  import RetroButton from '$lib/components/RetroButton.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let createNameVal = $state<string>((form?.createValues?.name as string) ?? '');
  let createSlugVal = $derived(
    String(createNameVal)
      .toLowerCase()
      .trim()
      .replace(/[^\w\s-]/g, '')
      .replace(/[\s_-]+/g, '-')
      .replace(/^-+|-+$/g, '')
  );

  let filtered = $derived(
    data.pokedexes.filter(p =>
      p.slug.includes(query.toLowerCase()) || p.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  function regionName(id: number | null): string {
    if (id == null) return '—';
    return data.regions.find(r => r.id === id)?.name ?? `#${id}`;
  }

  // Options mapping for selectors
  let regionOptions = $derived([
    { value: '', label: '— NONE —' },
    ...data.regions.map(r => ({ value: r.id, label: r.name.toUpperCase() }))
  ]);

  let copyFromOptions = $derived([
    { value: '', label: '— START EMPTY —' },
    ...data.pokedexes.map(p => ({
      value: p.id,
      label: `${p.name.toUpperCase()} (${p.source.toUpperCase()})`
    }))
  ]);
</script>

<svelte:head>
  <title>Edit Pokédexes | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>POKÉDEXES</h2>
    </div>
    <RetroButton type="button" variant="secondary" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW CUSTOM POKÉDEX'}
    </RetroButton>
  </header>

  {#if showCreate}
    <section class="create-section">
      <header class="section-header">
        <h3>CREATE CUSTOM POKÉDEX</h3>
      </header>
      <div class="section-body">
        <form action="?/create" method="post" use:enhance class="create-form">
          <input type="hidden" name="slug" value={createSlugVal} />
          <input type="hidden" name="isMainSeries" value="false" />

          <div class="field">
            <label for="c-name">NAME</label>
            <RetroInput id="c-name" name="name" type="text" maxlength={80} required bind:value={createNameVal} />
          </div>
          <div class="field">
            <label for="c-region">REGION</label>
            <RetroSelect id="c-region" name="regionId" value={form?.createValues?.regionId ?? ''} options={regionOptions} variant="editor" />
          </div>
          <div class="field">
            <label for="c-copy">COPY EXISTING POKÉDEX</label>
            <RetroSelect id="c-copy" name="copyFromId" value={form?.createValues?.copyFromId ?? ''} options={copyFromOptions} variant="editor" />
          </div>
          <div class="form-actions">
            <RetroButton type="submit" variant="primary">CREATE</RetroButton>
            <RetroButton
              type="button"
              variant="text"
              onclick={() => {
                showCreate = false;
              }}
            >
              CANCEL
            </RetroButton>
            {#if form?.createError}
              <span class="form-error">{form.createError}</span>
            {/if}
          </div>
        </form>
      </div>
    </section>
  {/if}

  <SearchBar bind:value={query} placeholder="Search pokédexes..." count={filtered.length} total={data.total} />

  {#if filtered.length === 0}
    <div class="empty-state">
      <p>No pokédexes yet.</p>
      <p class="hint">Click "+ NEW CUSTOM POKÉDEX" above to create the first one. Custom dexes are perfect for ROM hacks.</p>
    </div>
  {:else}
    <DataTable compact>
      {#snippet head()}
        <th>SLUG</th>
        <th>NAME</th>
        <th>REGION</th>
        <th style="text-align: right">ENTRIES</th>
        <th style="text-align: center">SOURCE</th>
        <th style="width: 80px; text-align: center"></th>
      {/snippet}
      {#each filtered as p (p.id)}
        <tr>
          <td class="row-slug">{p.slug}</td>
          <td class="row-name">{p.name.toUpperCase()}</td>
          <td class="row-region">{regionName(p.regionId).toUpperCase()}</td>
          <td class="row-entries" style="text-align: right">{p.entryCount}</td>
          <td style="text-align: center">
            <span class="src-badge src-{p.source}">{p.source.toUpperCase()}</span>
          </td>
          <td style="text-align: center">
            {#if p.source === 'custom'}
              <a href="/editor/pokedex/{p.slug}" class="edit-link edit-custom">EDIT →</a>
            {:else}
              <a href="/editor/pokedex/{p.slug}" class="edit-link edit-view">VIEW →</a>
            {/if}
          </td>
        </tr>
      {/each}
    </DataTable>
  {/if}
</div>

<style>
  .entity-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .create-section {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
  }

  .section-header {
    background-color: var(--theme-bg-tertiary);
    border-bottom: 4px solid var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-4);
  }

  .section-header h3 {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-inverse);
    margin: 0;
    text-transform: uppercase;
  }

  .section-body {
    padding: var(--spacing-4);
  }

  .create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--spacing-4);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
    justify-content: flex-end;
  }

  .field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
    min-height: 20px;
    display: flex;
    align-items: flex-end;
  }

  :global(.create-form .retro-input) {
    height: 32px !important;
    width: 100% !important;
    text-align: left !important;
    padding-left: 10px !important;
    font-family: var(--font-display) !important;
    font-size: 10px !important;
  }

  :global(.create-form .retro-select-wrapper) {
    width: 100% !important;
    height: 32px !important;
  }

  .form-actions {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
  }

  .row-slug {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
  }

  .row-name {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .row-region,
  .row-main,
  .row-entries {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .row-entries {
    color: var(--theme-text-primary);
  }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
    text-align: center;
    display: inline-block;
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-link {
    font-family: var(--font-display);
    font-size: 9px;
    text-decoration: none;
  }

  .edit-link.edit-custom {
    color: var(--theme-text-accent);
  }

  .edit-link.edit-view {
    color: var(--theme-text-muted);
  }

  .edit-link:hover {
    text-decoration: underline;
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
    margin: 0 0 var(--spacing-2) 0;
  }

  .empty-state .hint {
    font-size: 11px;
    color: var(--theme-text-muted);
  }
</style>
