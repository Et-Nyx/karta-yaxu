<script lang="ts">
  let dialogText = $state('A wild BATTLE MODE appeared!');

  const menuOptions = ['FIGHT', 'BAG', 'POKÉMON', 'RUN'];

  function handleOption(option: string) {
    if (option === 'RUN') {
      dialogText = "Can't escape! Battle Mode is under active construction.";
    } else if (option === 'BAG') {
      dialogText = "Your BAG is empty! Coding tools are still being crafted.";
    } else if (option === 'POKÉMON') {
      dialogText = "Your POKÉMON are resting inside the SQLite databases.";
    } else if (option === 'FIGHT') {
      dialogText = "TRAINER used CODE EDIT! It is super effective!";
    }
  }

  function resetDialog() {
    dialogText = 'A wild BATTLE MODE appeared!';
  }
</script>

<svelte:head>
  <title>Battle Mode | Karta Yaxu</title>
</svelte:head>

<div class="battle-scene-container">
  <div class="battle-scene">
    <!-- Enemy side -->
    <div class="enemy-side">
      <div class="status-box enemy-status">
        <div class="status-header">
          <span class="name">BATTLE MODE</span>
          <span class="level">Lv99</span>
        </div>
        <div class="hp-bar-container">
          <span class="hp-label">HP:</span>
          <div class="hp-bar-bg">
            <div class="hp-bar-fill work-in-progress" style="width: 80%"></div>
          </div>
        </div>
        <div class="status-text">STATUS: CODING</div>
      </div>
      
      <div class="sprite-placeholder enemy-sprite">
        <!-- SVG Game controller retro graphic representing Battle Mode -->
        <svg viewBox="0 0 64 64" class="battle-graphic">
          <rect x="12" y="20" width="40" height="24" fill="none" stroke="var(--theme-border-primary)" stroke-width="4" />
          <rect x="20" y="30" width="8" height="4" fill="var(--theme-text-accent)" />
          <rect x="22" y="28" width="4" height="8" fill="var(--theme-text-accent)" />
          <circle cx="40" cy="32" r="3" fill="var(--theme-border-primary)" />
          <circle cx="46" cy="32" r="3" fill="var(--theme-border-primary)" />
        </svg>
      </div>
    </div>

    <!-- Player side -->
    <div class="player-side">
      <div class="sprite-placeholder player-sprite">
        <!-- Trainer back sprite representation -->
        <svg viewBox="0 0 64 64" class="battle-graphic back-sprite">
          <rect x="24" y="16" width="16" height="20" fill="var(--theme-text-secondary)" />
          <rect x="16" y="36" width="32" height="28" fill="var(--theme-border-primary)" />
          <rect x="20" y="44" width="24" height="20" fill="var(--theme-bg-primary)" />
        </svg>
      </div>

      <div class="status-box player-status">
        <div class="status-header">
          <span class="name">TRAINER</span>
          <span class="level">Lv10</span>
        </div>
        <div class="hp-bar-container">
          <span class="hp-label">HP:</span>
          <div class="hp-bar-bg">
            <div class="hp-bar-fill" style="width: 100%"></div>
          </div>
        </div>
        <div class="hp-text">100 / 100</div>
      </div>
    </div>

    <!-- Console / Dialog panel -->
    <div class="battle-console">
      <button class="dialogue-box" onclick={resetDialog}>
        <p>{dialogText}</p>
        {#if dialogText !== 'A wild BATTLE MODE appeared!'}
          <span class="blink-arrow">▼</span>
        {/if}
      </button>

      <div class="menu-box">
        <div class="menu-grid">
          {#each menuOptions as opt}
            <button class="menu-btn" onclick={() => handleOption(opt)}>
              <span class="btn-bullet">▶</span> {opt}
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .battle-scene-container {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    padding: var(--spacing-6) 0;
  }

  .battle-scene {
    width: 100%;
    max-width: 680px;
    background-color: var(--theme-bg-secondary);
    border: 4px solid var(--theme-border-primary);
    box-shadow: var(--shadow-card);
    padding: var(--spacing-6);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-6);
    min-height: 420px;
    position: relative;
    box-sizing: border-box;
  }

  .enemy-side, .player-side {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    width: 100%;
  }

  .player-side {
    flex-direction: row-reverse;
  }

  .status-box {
    background-color: var(--theme-bg-primary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-3);
    width: 220px;
    box-shadow: 2px 2px 0 var(--theme-border-primary);
    box-sizing: border-box;
  }

  .status-header {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-display);
    font-size: 10px;
    font-weight: bold;
    color: var(--theme-text-primary);
    margin-bottom: var(--spacing-2);
  }

  .hp-bar-container {
    display: flex;
    align-items: center;
    gap: var(--spacing-2);
    margin-bottom: var(--spacing-2);
  }

  .hp-label {
    font-family: var(--font-display);
    font-size: 8px;
    color: var(--theme-text-secondary);
    font-weight: bold;
  }

  .hp-bar-bg {
    flex: 1;
    height: 8px;
    border: 2px solid var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
    position: relative;
  }

  .hp-bar-fill {
    height: 100%;
    background-color: var(--theme-text-accent);
  }

  .hp-bar-fill.work-in-progress {
    background-color: var(--theme-text-accent);
    animation: pulse 1.5s infinite ease-in-out;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.6; }
    50% { opacity: 1; }
  }

  .status-text, .hp-text {
    font-family: var(--font-mono);
    font-size: 9px;
    color: var(--theme-text-secondary);
    text-align: right;
  }

  .sprite-placeholder {
    width: 120px;
    height: 120px;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .battle-graphic {
    width: 80px;
    height: 80px;
  }

  .enemy-sprite .battle-graphic {
    animation: float 3s infinite ease-in-out;
  }

  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-8px); }
  }

  /* Console & dialogue grid */
  .battle-console {
    display: flex;
    border: 4px solid var(--theme-border-primary);
    background-color: var(--theme-bg-primary);
    height: 96px;
    box-shadow: 3px 3px 0 var(--theme-border-primary);
    overflow: hidden;
    box-sizing: border-box;
  }

  .dialogue-box {
    flex: 1.4;
    padding: var(--spacing-3) var(--spacing-4);
    font-family: var(--font-display);
    font-size: 11px;
    line-height: 1.4;
    color: var(--theme-text-primary);
    position: relative;
    cursor: pointer;
    border: none;
    border-right: 4px solid var(--theme-border-primary);
    background: none;
    text-align: left;
    outline: none;
  }

  .dialogue-box p {
    margin: 0;
  }

  .blink-arrow {
    position: absolute;
    right: 8px;
    bottom: 4px;
    font-size: 8px;
    animation: blink 0.8s infinite steps(2, start);
  }

  @keyframes blink {
    to { visibility: hidden; }
  }

  .menu-box {
    flex: 1;
    padding: var(--spacing-2);
    box-sizing: border-box;
  }

  .menu-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: var(--spacing-1);
    height: 100%;
  }

  .menu-btn {
    font-family: var(--font-display);
    font-size: 10px;
    color: var(--theme-text-primary);
    background: none;
    border: 2px solid transparent;
    text-align: left;
    padding: var(--spacing-1) var(--spacing-2);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--spacing-1);
    outline: none;
  }

  .menu-btn:hover {
    border-color: var(--theme-border-primary);
    background-color: var(--theme-bg-secondary);
  }

  .btn-bullet {
    visibility: hidden;
    color: var(--theme-text-accent);
  }

  .menu-btn:hover .btn-bullet {
    visibility: visible;
  }

  @media (max-width: 768px) {
    .battle-scene {
      padding: var(--spacing-4);
    }

    .enemy-side, .player-side {
      flex-direction: column;
      align-items: center;
      gap: var(--spacing-2);
    }
    
    .player-side {
      flex-direction: column-reverse;
    }

    .battle-console {
      flex-direction: column;
      height: auto;
    }

    .dialogue-box {
      border-right: none;
      border-bottom: 4px solid var(--theme-border-primary);
      padding: var(--spacing-4);
    }

    .menu-box {
      padding: var(--spacing-4);
    }

    .menu-grid {
      height: auto;
    }
  }
</style>
