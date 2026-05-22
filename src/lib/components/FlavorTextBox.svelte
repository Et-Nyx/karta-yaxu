<script lang="ts">
  let {
    text,
    editable = false,
    onchange,
    form,
    name
  }: {
    text: string;
    editable?: boolean;
    onchange?: (val: string) => void;
    form?: string;
    name?: string;
  } = $props();
</script>

<div class="flavor-text-box">
  {#if editable}
    <textarea
      {form}
      {name}
      class="retro-textarea"
      value={text}
      oninput={(e) => onchange?.((e.target as HTMLTextAreaElement).value)}
      placeholder="No Pokédex entry available for this version. Type here to add/edit…"
    ></textarea>
  {:else}
    <p>{text}</p>
  {/if}
</div>

<style>
  .flavor-text-box {
    background-color: var(--theme-bg-tertiary);
    border: 3px solid var(--theme-border-primary);
    padding: var(--spacing-4);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.5;
    color: var(--theme-text-inverse);
    box-shadow: 2px 2px 0px var(--theme-border-primary);
  }

  .flavor-text-box p { margin: 0; }

  .retro-textarea {
    width: 100%;
    min-height: 100px;
    background: transparent;
    border: none;
    outline: none;
    color: var(--theme-text-inverse);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 1.5;
    padding: 0;
    margin: 0;
    box-sizing: border-box;
    resize: vertical;
  }

  @media (max-width: 768px) {
    .flavor-text-box { font-size: 11px; line-height: 1.4; }
    .retro-textarea { font-size: 11px; line-height: 1.4; }
  }
</style>
