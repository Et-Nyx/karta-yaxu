<script lang="ts">
  import DataTable from '$lib/components/DataTable.svelte';

  let { data } = $props();

  const statLabel: Record<string, string> = {
    'hp':              'HP',
    'attack':          'ATTACK',
    'defense':         'DEFENSE',
    'special-attack':  'SP. ATK',
    'special-defense': 'SP. DEF',
    'speed':           'SPEED',
  };

  const statVar: Record<string, string> = {
    'hp':              '--color-stat-hp',
    'attack':          '--color-stat-atk',
    'defense':         '--color-stat-def',
    'special-attack':  '--color-stat-spa',
    'special-defense': '--color-stat-spd',
    'speed':           '--color-stat-spe',
  };

  function label(stat: string | null): string {
    return stat ? (statLabel[stat] ?? stat.toUpperCase()) : '—';
  }

  function isNeutral(n: typeof data.natures[0]): boolean {
    return !n.increasedStat && !n.decreasedStat;
  }
</script>

<svelte:head>
  <title>Natures | Karta Yaxu</title>
</svelte:head>

<div class="natures-page">
  <div class="page-header">
    <h2>NATURES</h2>
    <p class="subtitle">25 natures — affects stat growth and flavor preferences.</p>
  </div>

  <DataTable>
    {#snippet head()}
      <th>NAME</th>
      <th style="text-align: center">STAT ↑</th>
      <th style="text-align: center">STAT ↓</th>
      <th style="text-align: center">LIKES</th>
      <th style="text-align: center">HATES</th>
    {/snippet}
    {#each data.natures as n (n.id)}
      <tr class:neutral={isNeutral(n)}>
        <td class="name-cell">{n.name.toUpperCase()}</td>
        <td class="stat-cell">
          {#if n.increasedStat}
            <span class="stat-tag up" style="color: var({statVar[n.increasedStat] ?? '--theme-text-primary'})">
              {label(n.increasedStat)}
            </span>
          {:else}
            <span class="neutral-dash">—</span>
          {/if}
        </td>
        <td class="stat-cell">
          {#if n.decreasedStat}
            <span class="stat-tag down">
              {label(n.decreasedStat)}
            </span>
          {:else}
            <span class="neutral-dash">—</span>
          {/if}
        </td>
        <td class="flavor-cell">{n.likesFlavor ? n.likesFlavor.toUpperCase() : '—'}</td>
        <td class="flavor-cell">{n.hatesFlavor ? n.hatesFlavor.toUpperCase() : '—'}</td>
      </tr>
    {/each}
  </DataTable>
</div>

<style>
  .natures-page {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
  }

  .page-header h2 {
    font-family: var(--font-display);
    font-size: 20px;
    color: var(--theme-text-primary);
    margin: 0 0 var(--spacing-2) 0;
  }

  .subtitle {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--theme-text-secondary);
    margin: 0;
  }

  :global(tr.neutral) { opacity: 0.55; }

  .name-cell {
    font-family: var(--font-display);
    font-size: 10px;
    white-space: nowrap;
  }

  .stat-cell { text-align: center; }

  .stat-tag {
    font-family: var(--font-display);
    font-size: 9px;
    padding: 2px 6px;
    display: inline-block;
  }

  .stat-tag.up {
    background-color: color-mix(in srgb, currentColor 12%, transparent);
  }

  .stat-tag.down {
    color: var(--color-text-error);
    background-color: color-mix(in srgb, var(--color-text-error) 12%, transparent);
  }

  .neutral-dash { color: var(--theme-text-muted); }

  .flavor-cell {
    text-align: center;
    font-size: 12px;
    color: var(--theme-text-secondary);
  }
</style>
