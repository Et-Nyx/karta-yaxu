<script lang="ts">
  import { enhance } from '$app/forms';
  import TypeIcon from '$lib/components/TypeIcon.svelte';
  import TypeBadge from '$lib/components/TypeBadge.svelte';
  import PokemonCard from '$lib/components/PokemonCard.svelte';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import { pageSource } from '$lib/stores/pageSource';

  let { data, form } = $props();

  $effect(() => { pageSource.set(data.type.source); });

  let saved = $state(false);

  function handleSave() {
    saved = false;
    return async ({ result, update }: any) => {
      await update();
      if (result.type === 'success') {
        saved = true;
        setTimeout(() => saved = false, 2500);
      }
    };
  }

  let nameVal = $state(data.type.name ?? '');
  let slugVal = $state(data.type.slug ?? '');
  let hexColorVal = $state(data.type.hexColor ?? '');
  let iconUrlVal = $state(data.type.iconUrl ?? '');
  let dmgClassVal = $state(data.type.moveDamageClass ?? '');
  let genIdVal = $state(data.type.generationId ?? null);

  $effect(() => {
    const src = form?.values ?? data.type;
    nameVal = (src.name as string) ?? '';
    slugVal = (src.slug as string) ?? '';
    hexColorVal = (src.hexColor as string) ?? '';
    iconUrlVal = (src.iconUrl as string) ?? '';
    dmgClassVal = (src.moveDamageClass as string) ?? '';
    genIdVal = (src.generationId as number | null) ?? null;
  });

  let doubleDamageTo = $derived(data.attackingList.filter((x: { multiplier: number }) => x.multiplier === 2.0));
  let halfDamageTo = $derived(data.attackingList.filter((x: { multiplier: number }) => x.multiplier === 0.5));
  let noDamageTo = $derived(data.attackingList.filter((x: { multiplier: number }) => x.multiplier === 0.0));

  let doubleDamageFrom = $derived(data.defendingList.filter((x: { multiplier: number }) => x.multiplier === 2.0));
  let halfDamageFrom = $derived(data.defendingList.filter((x: { multiplier: number }) => x.multiplier === 0.5));
  let noDamageFrom = $derived(data.defendingList.filter((x: { multiplier: number }) => x.multiplier === 0.0));

  let searchQuery = $state('');

  let filteredPokemon = $derived(
    data.pokemonList.filter((p: { name: string; pokeapiId: number | null }) =>
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.pokeapiId?.toString() === searchQuery
    )
  );
</script>

<svelte:head>
  <title>Edit {data.type.name} Type | Karta Yaxu</title>
</svelte:head>

<div class="type-detail-page">
  <form action="?/update" method="post" use:enhance={handleSave}>
    <header class="detail-header">
      <div class="header-content">
        <TypeIcon typeName={nameVal} size="lg" />
        <div class="header-title">
          <input class="type-label-input" name="name" bind:value={nameVal} maxlength="50" required />
          <h1>TYPE</h1>
        </div>
      </div>
      <div class="header-bar" style="background-color: {hexColorVal ? hexColorVal : `var(--color-type-${nameVal})`}"></div>
      <div class="meta-strip">
        <div class="meta-field"><label>SLUG</label><input name="slug" type="text" bind:value={slugVal}></div>
        <div class="meta-field">
          <label>HEX COLOR</label>
          <div class="hex-row">
            {#if hexColorVal}<span class="swatch" style="background-color: {hexColorVal}"></span>{/if}
            <input name="hexColor" type="text" bind:value={hexColorVal} placeholder="#FF0000">
          </div>
        </div>
        <div class="meta-field"><label>ICON URL</label><input name="iconUrl" type="text" bind:value={iconUrlVal}></div>
        <div class="meta-field">
          <label>DMG CLASS</label>
          <select name="moveDamageClass" bind:value={dmgClassVal}>
            <option value="">— none —</option>
            {#each ['physical','special','status'] as v}<option value={v} selected={dmgClassVal===v}>{v}</option>{/each}
          </select>
        </div>
        <div class="meta-field"><label>GEN ID</label><input name="generationId" type="number" min="1" bind:value={genIdVal}></div>
        {#if data.type.pokeapiId}<span class="meta-id">PokéAPI: {data.type.pokeapiId}</span>{/if}
        <div class="meta-actions">
          <button type="submit" class="save-btn">SAVE</button>
          <a href="." class="cancel-link">CANCEL</a>
          {#if saved}<span class="save-ok">✓ SAVED</span>{/if}
          {#if form?.updateError}<span class="form-error">{form.updateError}</span>{/if}
        </div>
      </div>
    </header>
  </form>

  <!-- Damage Matchups (read-only) -->
  <section class="matchups-grid">
    <div class="matchup-box">
      <h3>OFFENSIVE (ATTACKING)</h3>

      <div class="matchup-row">
        <span class="label super">SUPER EFFECTIVE (2x):</span>
        <div class="badges">
          {#each doubleDamageTo as m} <TypeBadge name={m.defenderName} hexColor={m.defenderHexColor} size="sm" /> {/each}
          {#if doubleDamageTo.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>

      <div class="matchup-row">
        <span class="label not-very">NOT VERY EFFECTIVE (0.5x):</span>
        <div class="badges">
          {#each halfDamageTo as m} <TypeBadge name={m.defenderName} hexColor={m.defenderHexColor} size="sm" /> {/each}
          {#if halfDamageTo.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>

      <div class="matchup-row">
        <span class="label immune">NO EFFECT (0x):</span>
        <div class="badges">
          {#each noDamageTo as m} <TypeBadge name={m.defenderName} hexColor={m.defenderHexColor} size="sm" /> {/each}
          {#if noDamageTo.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
    </div>

    <div class="matchup-box">
      <h3>DEFENSIVE (DEFENDING)</h3>

      <div class="matchup-row">
        <span class="label super">WEAK TO (2x):</span>
        <div class="badges">
          {#each doubleDamageFrom as m} <TypeBadge name={m.attackerName} hexColor={m.attackerHexColor} size="sm" /> {/each}
          {#if doubleDamageFrom.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>

      <div class="matchup-row">
        <span class="label not-very">RESISTS (0.5x):</span>
        <div class="badges">
          {#each halfDamageFrom as m} <TypeBadge name={m.attackerName} hexColor={m.attackerHexColor} size="sm" /> {/each}
          {#if halfDamageFrom.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>

      <div class="matchup-row">
        <span class="label immune">IMMUNE TO (0x):</span>
        <div class="badges">
          {#each noDamageFrom as m} <TypeBadge name={m.attackerName} hexColor={m.attackerHexColor} size="sm" /> {/each}
          {#if noDamageFrom.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
    </div>
  </section>

  <!-- Pokemon list (read-only) -->
  <section class="pokemon-section">
    <div class="section-header">
      <h2>POKÉMON OF THIS TYPE</h2>
      <SearchBar bind:value={searchQuery} placeholder="Filter Pokémon..." count={filteredPokemon.length} total={data.pokemonList.length} />
    </div>

    <div class="pokemon-grid">
      {#each filteredPokemon as p (p.id)}
        <PokemonCard
          pokemon={{
            pokeapiId: p.pokeapiId || 0,
            name: p.name,
            slug: p.slug,
            spriteUrl: p.spriteDefault,
            types: p.types
          }}
        />
      {/each}
    </div>
  </section>

  {#if data.type.source === 'edited' && data.type.pokeapiId}
    <form action="?/resource" method="post" use:enhance class="resource-form">
      <button type="submit" class="resource-btn" onclick={(e) => { if (!confirm('Re-import this type from PokeAPI? All your edits will be lost.')) e.preventDefault(); }}>
        ⟲ RE-SOURCE FROM POKEAPI
      </button>
      <span class="resource-hint">Reverts to canonical PokeAPI data; source flips back to 'pokeapi'.</span>
    </form>
  {/if}

  {#if data.type.source === 'custom'}
    <form action="?/delete" method="post" use:enhance class="delete-form">
      <button type="submit" class="delete-btn">DELETE THIS TYPE</button>
      <span class="delete-hint">Apenas registros custom podem ser deletados.</span>
    </form>
  {/if}
</div>

<style>
  .type-detail-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }


  /* ── Header ── */

  .detail-header {
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: var(--spacing-4);
    padding: var(--spacing-6);
  }

  .header-title {
    display: flex;
    align-items: baseline;
    gap: var(--spacing-3);
  }

  .type-label-input {
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-secondary);
    letter-spacing: 2px;
    background: none;
    border: none;
    border-bottom: 1px solid transparent;
    outline: none;
    padding: 0;
    width: auto;
  }

  .type-label-input:focus {
    border-bottom-color: var(--theme-text-accent);
  }

  h1 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--theme-text-primary);
    margin: 0;
  }

  .header-bar {
    height: 4px;
    width: 100%;
    flex-shrink: 0;
  }

  /* ── Meta strip ── */

  .meta-strip {
    border-top: 2px dashed var(--theme-border-primary);
    padding: var(--spacing-3) var(--spacing-6);
    display: flex;
    gap: var(--spacing-4);
    align-items: center;
    flex-wrap: wrap;
  }

  .meta-field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-field label {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-muted);
  }

  .meta-field input,
  .meta-field select {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--theme-bg-primary);
    border: 1px solid var(--theme-border-primary);
    color: var(--theme-text-primary);
    padding: 2px var(--spacing-1);
  }

  .hex-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
  }

  .swatch {
    display: inline-block;
    width: 18px;
    height: 18px;
    border: 1px solid var(--theme-border-primary);
    flex-shrink: 0;
  }

  .meta-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
    margin-left: auto;
  }

  .save-btn {
    font-family: var(--font-display);
    font-size: 11px;
    padding: var(--spacing-1) var(--spacing-3);
    background: none;
    border: 2px solid var(--theme-text-accent);
    color: var(--theme-text-accent);
    cursor: pointer;
  }

  .save-btn:hover {
    background-color: var(--theme-text-accent);
    color: var(--theme-bg-primary);
  }

  .cancel-link {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-muted);
    text-decoration: none;
  }

  .cancel-link:hover {
    text-decoration: underline;
  }

  .save-ok {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-accent);
  }

  .form-error {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--color-text-error);
  }

  .src-badge {
    font-family: var(--font-display);
    font-size: 8px;
    padding: 2px 6px;
    border: 2px solid var(--theme-border-primary);
  }

  .src-pokeapi { color: var(--theme-text-muted); }
  .src-edited { color: var(--theme-text-accent); border-color: var(--theme-text-accent); }
  .src-custom { color: var(--color-text-warning); border-color: var(--color-text-warning); }

  .meta-id {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--theme-text-muted);
  }

  .wiki-link {
    font-family: var(--font-display);
    font-size: 9px;
    color: var(--theme-text-accent);
    text-decoration: none;
  }

  .wiki-link:hover {
    text-decoration: underline;
  }

  /* ── Matchups ── */

  .matchups-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
  }

  @media (max-width: 768px) {
    .matchups-grid {
      grid-template-columns: 1fr;
    }
  }

  .matchup-box {
    background-color: var(--theme-bg-secondary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .matchup-box h3 {
    font-family: var(--font-display);
    font-size: 14px;
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-2) 0;
    border-bottom: 2px dashed var(--theme-border-primary);
    padding-bottom: var(--spacing-2);
  }

  .matchup-row {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-2);
  }

  .matchup-row .label {
    font-family: var(--font-display);
    font-size: 10px;
  }

  .label.super { color: var(--color-effective-positive); }
  .label.not-very { color: var(--color-effective-negative); }
  .label.immune { color: var(--theme-text-secondary); }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-2);
  }

  .none {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--theme-text-secondary);
  }

  /* ── Pokemon section ── */

  .pokemon-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-4);
  }

  .section-header h2 {
    color: var(--theme-text-primary);
    margin: 0;
    font-size: 18px;
  }

  .pokemon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--spacing-4);
  }

  /* ── Resource / delete forms ── */

  .resource-form, .delete-form {
    display: flex;
    align-items: center;
    gap: var(--spacing-3);
  }

  .resource-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--theme-text-muted);
    color: var(--theme-text-muted);
    cursor: pointer;
  }

  .resource-btn:hover {
    border-color: var(--theme-text-accent);
    color: var(--theme-text-accent);
  }

  .resource-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  .delete-btn {
    font-family: var(--font-display);
    font-size: 10px;
    padding: var(--spacing-2) var(--spacing-4);
    background: none;
    border: 2px solid var(--color-text-error);
    color: var(--color-text-error);
    cursor: pointer;
  }

  .delete-btn:hover {
    background-color: var(--color-text-error);
    color: var(--theme-bg-primary);
  }

  .delete-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--theme-text-muted);
  }

  @media (max-width: 768px) {
    .header-content {
      padding: var(--spacing-4);
    }

    h1 {
      font-size: 18px;
    }

    .type-label-input {
      font-size: 12px;
    }

    .meta-strip {
      flex-direction: column;
      align-items: flex-start;
      padding: var(--spacing-3) var(--spacing-4);
    }

    .meta-actions {
      margin-left: 0;
    }

    .section-header { align-items: flex-start; }
  }
</style>
