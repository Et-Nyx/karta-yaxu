<script lang="ts">
  import TypeBadge from '$lib/components/TypeBadge.svelte';

  interface MatchupEntry { name: string; multiplier: number; hexColor: string | null; }

  let {
    offensive,
    defensive,
  }: { offensive: MatchupEntry[]; defensive: MatchupEntry[]; } = $props();

  const offGroups = $derived([
    { label: 'SUPER EFFECTIVE (2×)', cls: 'super',    entries: offensive.filter(e => e.multiplier >= 2) },
    { label: 'NOT VERY EFFECTIVE (½×)', cls: 'not-very', entries: offensive.filter(e => e.multiplier > 0 && e.multiplier < 1) },
    { label: 'NO EFFECT (0×)',      cls: 'immune',   entries: offensive.filter(e => e.multiplier === 0) },
  ]);

  const defGroups = $derived(() => {
    const quadWeak   = defensive.filter(e => e.multiplier >= 4);
    const weak       = defensive.filter(e => e.multiplier >= 2 && e.multiplier < 4);
    const resist     = defensive.filter(e => e.multiplier > 0 && e.multiplier < 1 && e.multiplier >= 0.5);
    const quadResist = defensive.filter(e => e.multiplier > 0 && e.multiplier < 0.5);
    const immune     = defensive.filter(e => e.multiplier === 0);

    const groups: { label: string; cls: string; entries: MatchupEntry[] }[] = [];
    if (quadWeak.length > 0) groups.push({ label: 'WEAK TO (4×)',   cls: 'quad-weak',   entries: quadWeak });
    groups.push({ label: 'WEAK TO (2×)',    cls: 'super',      entries: weak });
    groups.push({ label: 'RESISTS (½×)',    cls: 'not-very',   entries: resist });
    if (quadResist.length > 0) groups.push({ label: 'RESISTS (¼×)', cls: 'quad-resist', entries: quadResist });
    groups.push({ label: 'IMMUNE TO (0×)', cls: 'immune',     entries: immune });
    return groups;
  });
</script>

<div class="matchup-grid">
  <div class="matchup-box">
    <h3>OFFENSIVE (ATTACKING)</h3>
    {#each offGroups as group}
      <div class="matchup-row">
        <span class="label {group.cls}">{group.label}</span>
        <div class="badges">
          {#each group.entries as e}<TypeBadge name={e.name} hexColor={e.hexColor} size="sm" />{/each}
          {#if group.entries.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
    {/each}
  </div>

  <div class="matchup-box">
    <h3>DEFENSIVE (DEFENDING)</h3>
    {#each defGroups() as group}
      <div class="matchup-row">
        <span class="label {group.cls}">{group.label}</span>
        <div class="badges">
          {#each group.entries as e}<TypeBadge name={e.name} hexColor={e.hexColor} size="sm" />{/each}
          {#if group.entries.length === 0}<span class="none">None</span>{/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .matchup-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-6);
  }

  @media (max-width: 768px) {
    .matchup-grid { grid-template-columns: 1fr; }
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

  .label.super       { color: var(--color-effective-positive); }
  .label.quad-weak   { color: var(--color-effective-quad-weak); }
  .label.not-very    { color: var(--color-effective-negative); }
  .label.quad-resist { color: var(--color-effective-quad-resist); }
  .label.immune     { color: var(--theme-text-secondary); }

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
</style>
