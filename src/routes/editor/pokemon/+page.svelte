<script lang="ts">
  import { enhance } from '$app/forms';
  import SearchBar from '$lib/components/SearchBar.svelte';

  let { data, form } = $props();

  let query = $state('');
  let showCreate = $state(false);
  let prefillFetched = $state(false);
  let prefillMsg = $state('');

  let filtered = $derived(
    data.pokemon.filter(p =>
      p.slug.includes(query.toLowerCase()) || p.name.toLowerCase().includes(query.toLowerCase())
    )
  );

  function dexFor(speciesId: number): number | null {
    return data.species.find(s => s.id === speciesId)?.dexNumber ?? null;
  }

  function toggleCreate() {
    showCreate = !showCreate;
    // Reset prefill state when reopening so a fresh species pick prefills again
    if (showCreate) {
      prefillFetched = false;
      prefillMsg = '';
    }
  }

  async function onSpeciesChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const speciesId = target.value;
    if (!speciesId || prefillFetched) return;

    try {
      const res = await fetch(`/api/editor/base-form/${speciesId}`);
      const body = await res.json();
      if (!body.found) {
        prefillMsg = 'No Base Form found for this species — fill manually.';
        return;
      }

      const form = target.form as HTMLFormElement;
      const isEmpty = (v: string) => v === '' || v === '0';
      for (const [key, val] of Object.entries(body.pokemon)) {
        const input = form.elements.namedItem(key) as HTMLInputElement | null;
        if (input && isEmpty(input.value) && val != null) {
          input.value = String(val);
        }
      }
      prefillFetched = true;
      prefillMsg = `Prefilled from Base Form (${body.types.length} types, ${body.abilities.length} abilities, learnset copied on save)`;
    } catch (err) {
      prefillMsg = 'Failed to fetch Base Form.';
    }
  }
</script>

<svelte:head>
  <title>Edit Pokémon | Karta Yaxu</title>
</svelte:head>

<div class="entity-page">
  <header class="page-header">
    <div>
      <h2>POKÉMON</h2>
      <span class="count">{data.total} total · {filtered.length} shown</span>
    </div>
    <button type="button" class="new-btn" onclick={toggleCreate}>
      {showCreate ? '× CANCEL' : '+ NEW POKÉMON'}
    </button>
  </header>

  {#if showCreate}
    <section class="create-section">
      <h3 class="section-title">CREATE POKÉMON</h3>
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
          <label for="c-species">SPECIES</label>
          <select id="c-species" name="speciesId" required onchange={onSpeciesChange}>
            <option value="">— pick a species —</option>
            {#each data.species as s}
              <option value={s.id} selected={String(form?.createValues?.speciesId ?? '') === String(s.id)}>
                #{String(s.dexNumber).padStart(4, '0')} {s.name}
              </option>
            {/each}
          </select>
          {#if prefillMsg}<span class="prefill-msg">{prefillMsg}</span>{/if}
        </div>
        <div class="field">
          <label for="c-height">HEIGHT</label>
          <input id="c-height" name="height" type="number" min="0" required value={form?.createValues?.height ?? 0}>
        </div>
        <div class="field">
          <label for="c-weight">WEIGHT</label>
          <input id="c-weight" name="weight" type="number" min="0" required value={form?.createValues?.weight ?? 0}>
        </div>
        <div class="stats-row">
          <div class="field stat-field">
            <label for="c-hp">HP</label>
            <input id="c-hp" name="hp" type="number" min="0" max="255" required value={form?.createValues?.hp ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-atk">ATK</label>
            <input id="c-atk" name="atk" type="number" min="0" max="255" required value={form?.createValues?.atk ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-def">DEF</label>
            <input id="c-def" name="def" type="number" min="0" max="255" required value={form?.createValues?.def ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-spa">SPA</label>
            <input id="c-spa" name="spa" type="number" min="0" max="255" required value={form?.createValues?.spa ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-spd">SPD</label>
            <input id="c-spd" name="spd" type="number" min="0" max="255" required value={form?.createValues?.spd ?? 0}>
          </div>
          <div class="field stat-field">
            <label for="c-spe">SPE</label>
            <input id="c-spe" name="spe" type="number" min="0" max="255" required value={form?.createValues?.spe ?? 0}>
          </div>
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

  <SearchBar bind:value={query} placeholder="Search pokémon..." count={filtered.length} total={data.total} />

  {#if filtered.length === 0}
    <div class="empty-state">
      <p>No pokémon yet.</p>
      <p class="hint">Click "+ NEW POKÉMON" above to create the first one.</p>
    </div>
  {:else}
    <div class="entity-table">
      <div class="table-header">
        <span>SPRITE</span>
        <span>DEX</span>
        <span>SLUG</span>
        <span>NAME</span>
        <span>STATS</span>
        <span>SOURCE</span>
        <span></span>
      </div>
      {#each filtered as p (p.id)}
        <div class="table-row">
          <span class="row-sprite">
            {#if p.spriteDefault}
              <img src={p.spriteDefault} alt={p.name} loading="lazy">
            {:else}
              <span class="no-sprite">—</span>
            {/if}
          </span>
          <span class="row-dex">{dexFor(p.speciesId) != null ? `#${String(dexFor(p.speciesId)).padStart(4, '0')}` : '—'}</span>
          <span class="row-slug">{p.slug}</span>
          <span class="row-name">{p.name}</span>
          <span class="row-stats">{p.hp}/{p.atk}/{p.def}/{p.spa}/{p.spd}/{p.spe}</span>
          <span class="src-badge src-{p.source}">{p.source}</span>
          <a href="/editor/pokemon/{p.slug}" class="edit-link">EDIT →</a>
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

  .stats-row {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: var(--spacing-2);
  }

  .stat-field input {
    text-align: center;
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

  .prefill-msg {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-accent);
    margin-top: var(--spacing-1);
  }

  .entity-table {
    display: flex;
    flex-direction: column;
    border: 3px solid var(--theme-border-primary);
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 50px 70px 1fr 1fr 1.4fr 80px 60px;
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

  .table-row:last-child { border-bottom: none; }

  .table-row:hover {
    background-color: var(--theme-bg-hover);
  }

  .row-sprite {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .row-sprite img {
    width: 40px;
    height: 40px;
    image-rendering: pixelated;
    object-fit: contain;
  }

  .no-sprite {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .row-dex,
  .row-slug,
  .row-stats {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-secondary);
  }

  .row-name {
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--theme-text-primary);
  }

  .row-stats {
    color: var(--theme-text-muted);
  }

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
