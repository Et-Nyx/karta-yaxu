<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.moves.filter(m =>
      m.slug.includes(query.toLowerCase()) || m.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  function typeName(id: number | null): string {
    if (id == null) return '—';
    return data.types.find(t => t.id === id)?.name ?? `#${id}`;
  }

  function typeSlug(id: number | null): string {
    if (id == null) return '';
    return data.types.find(t => t.id === id)?.slug ?? '';
  }

  function dmgLetter(dc: string | null): string {
    if (dc === 'physical') return 'P';
    if (dc === 'special')  return 'S';
    if (dc === 'status')   return 'X';
    return '—';
  }
</script>

<svelte:head>
  <title>Edit Moves | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>MOVES</h2>
      <span class="count">{data.total} total · {filtered.length} shown</span>
    </div>
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW MOVE'}
    </button>
  </header>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE MOVE</h3>
      <form action="?/create" method="post" use:enhance class="create-form">
        <div class="field">
          <label for="c-name">NAME</label>
          <input id="c-name" name="name" type="text" maxlength="80" required value={form?.createValues?.name ?? ''}>
        </div>
        <div class="field">
          <label for="c-slug">SLUG</label>
          <input id="c-slug" name="slug" type="text" maxlength="80" pattern="[a-z0-9-]+" required value={form?.createValues?.slug ?? ''}>
        </div>
        <div class="field">
          <label for="c-type">TYPE</label>
          <select id="c-type" name="typeId" required>
            <option value="">— pick a type —</option>
            {#each data.types as t}
              <option value={t.id} selected={String(form?.createValues?.typeId ?? '') === String(t.id)}>{t.name}</option>
            {/each}
          </select>
        </div>
        <div class="field">
          <label for="c-dmg">DAMAGE CLASS</label>
          <select id="c-dmg" name="damageClass" required>
            <option value="physical" selected={(form?.createValues?.damageClass ?? 'physical') === 'physical'}>physical</option>
            <option value="special"  selected={form?.createValues?.damageClass === 'special'}>special</option>
            <option value="status"   selected={form?.createValues?.damageClass === 'status'}>status</option>
          </select>
        </div>
        <div class="field">
          <label for="c-power">POWER</label>
          <input id="c-power" name="power" type="number" min="0" value={form?.createValues?.power ?? ''}>
        </div>
        <div class="field">
          <label for="c-acc">ACCURACY</label>
          <input id="c-acc" name="accuracy" type="number" min="0" max="100" value={form?.createValues?.accuracy ?? ''}>
        </div>
        <div class="field">
          <label for="c-pp">PP</label>
          <input id="c-pp" name="pp" type="number" min="0" required value={form?.createValues?.pp ?? ''}>
        </div>
        <div class="field">
          <label for="c-prio">PRIORITY</label>
          <input id="c-prio" name="priority" type="number" value={form?.createValues?.priority ?? '0'}>
        </div>
        <div class="field">
          <label for="c-target">TARGET</label>
          <input id="c-target" name="target" type="text" maxlength="80" value={form?.createValues?.target ?? ''}>
        </div>
        <div class="field">
          <label for="c-gen">GENERATION</label>
          <select id="c-gen" name="generationId">
            <option value="">— none —</option>
            {#each data.generations as g}
              <option value={g.id} selected={String(form?.createValues?.generationId ?? '') === String(g.id)}>{g.name}</option>
            {/each}
          </select>
        </div>
        <div class="form-actions">
          <button type="submit" class="save-btn">CREATE</button>
          {#if form?.createError}
            <span class="form-error">{form.createError}</span>
          {/if}
        </div>
      </form>
    </section>
  {/if}

  <SearchBar bind:value={query} placeholder="Search moves..." count={filtered.length} total={data.total} />

  {#if filtered.length === 0}
    <div class="empty-state">
      <p>No moves yet.</p>
      <p class="hint">Click "+ NEW MOVE" above to create the first one.</p>
    </div>
  {:else}
    <div class="entity-table">
      <div class="table-header">
        <span>SLUG</span>
        <span>NAME</span>
        <span>TYPE</span>
        <span>DMG</span>
        <span>POWER</span>
        <span>ACC</span>
        <span>PP</span>
        <span>SOURCE</span>
        <span></span>
      </div>
      {#each filtered as m (m.id)}
        <div class="table-row">
          <span class="row-slug">{m.slug}</span>
          <span class="row-name">{m.name}</span>
          <span class="row-type type-{typeSlug(m.typeId)}">{typeName(m.typeId)}</span>
          <span class="row-dmg dmg-{m.damageClass}">{dmgLetter(m.damageClass)}</span>
          <span class="row-num">{m.power ?? '—'}</span>
          <span class="row-num">{m.accuracy ?? '—'}</span>
          <span class="row-num">{m.pp}</span>
          <span class="src-badge src-{m.source}">{m.source}</span>
          <a href="/editor/moves/{m.slug}" class="edit-link">EDIT →</a>
        </div>
      {/each}
    </div>
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

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .new-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    cursor: pointer;
  }

  .new-btn:hover {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .create-section {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-secondary);
    margin: 0 0 var(--spacing-3) 0;
  }

  .create-form {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-1);
  }

  .field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .field input,
  .field select {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: var(--spacing-2);
    background-color: var(--theme-bg-primary);
    border: 2px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
  }

  .field input:focus,
  .field select:focus {
    outline: none;
    border-color: var(--theme-text-accent);
  }

  .form-actions {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .save-btn {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
  }

  .save-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
  }

  .entity-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 1.4fr 1.4fr 90px 40px 60px 60px 50px 80px 60px;
    gap: var(--spacing-3);
    padding: var(--spacing-2) var(--spacing-3);
    align-items: center;
  }

  .table-header {
    background-color: var(--theme-bg-secondary);
    border-bottom: 2px solid var(--theme-border-primary);
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .table-row {
    border-bottom: 1px solid var(--theme-border-primary);
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background-color: var(--theme-bg-hover);
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

  .row-type {
    font-family: var(--font-display);
    font-size: 10px;
    text-transform: uppercase;
  }

  .row-dmg {
    font-family: var(--font-display);
    font-size: 11px;
    text-align: center;
  }

  .dmg-physical { color: var(--color-text-warning); }
  .dmg-special  { color: var(--theme-text-accent); }
  .dmg-status   { color: var(--theme-text-muted); }

  .row-num {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
    text-align: right;
  }

  /* Type colors — match the wiki theme; fall back to muted */
  .type-normal   { color: #a8a878; }
  .type-fire     { color: #f08030; }
  .type-water    { color: #6890f0; }
  .type-electric { color: #f8d030; }
  .type-grass    { color: #78c850; }
  .type-ice      { color: #98d8d8; }
  .type-fighting { color: #c03028; }
  .type-poison   { color: #a040a0; }
  .type-ground   { color: #e0c068; }
  .type-flying   { color: #a890f0; }
  .type-psychic  { color: #f85888; }
  .type-bug      { color: #a8b820; }
  .type-rock     { color: #b8a038; }
  .type-ghost    { color: #705898; }
  .type-dragon   { color: #7038f8; }
  .type-dark     { color: #705848; }
  .type-steel    { color: #b8b8d0; }
  .type-fairy    { color: #ee99ac; }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
    text-align: center;
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited  { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom  { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .edit-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    text-decoration: none;
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
