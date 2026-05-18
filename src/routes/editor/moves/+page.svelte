<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import MoveCard from '$lib/components/MoveCard.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);

  let filtered = $derived(
    data.moves.filter(m =>
      m.slug.includes(query.toLowerCase()) || m.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  function getType(typeId: number | null) {
    if (typeId == null) return null;
    return data.types.find(t => t.id === typeId) ?? null;
  }
</script>

<svelte:head>
  <title>Edit Moves | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <div class="page-header">
    <h2>MOVES</h2>
    <SearchBar bind:value={query} placeholder="Search moves..." count={filtered.length} total={data.total} />
  </div>

  <div class="header-actions">
    <button type="button" class="new-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '× CANCEL' : '+ NEW MOVE'}
    </button>
  </div>

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

  {#if filtered.length === 0}
    <div class="empty-state">
      <p>No moves yet.</p>
      <p class="hint">Click "+ NEW MOVE" above to create the first one.</p>
    </div>
  {:else}
    <div class="moves-list">
      {#each filtered as m (m.id)}
        {@const t = getType(m.typeId)}
        <div class="card-wrapper">
          <MoveCard
            name={m.name}
            slug={m.slug}
            typeName={t?.name ?? 'normal'}
            typeHexColor={null}
            damageClass={m.damageClass ?? 'physical'}
            power={m.power}
            accuracy={m.accuracy}
            pp={m.pp}
          />
          <a href="/editor/moves/{m.slug}" class="card-link-overlay" aria-label="Edit {m.name}"></a>
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
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .page-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-4);
  }

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .new-btn {
    font-family: var(--font-display);
    font-size: 9px;
    padding: var(--spacing-2) var(--spacing-4);
    background-color: var(--theme-bg-secondary);
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

  .moves-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-3);
  }

  .card-wrapper {
    position: relative;
  }

  .card-link-overlay {
    position: absolute;
    inset: 0;
    z-index: 1;
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
