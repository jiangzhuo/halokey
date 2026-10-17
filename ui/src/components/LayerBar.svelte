<script lang="ts">
  import type { Snippet } from "svelte";
  let { layer, onSelect, right }: { layer: number; onSelect: (l: number) => void; right?: Snippet } = $props();
</script>
<div class="bar">
  <span class="muted">Layer</span>
  <div class="seg">{#each [0, 1, 2, 3] as l}<button class:on={layer === l} onclick={() => onSelect(l)}>{l}</button>{/each}</div>
  <div class="right">{@render right?.()}</div>
</div>
<style>
  .bar { display: flex; align-items: flex-start; gap: 14px; padding: 12px 22px; }
  .bar > .muted, .bar > .seg { margin-top: 4px; }
  .muted { color: var(--muted); font-size: 13px; }
  .seg { display: inline-flex; background: var(--card); border-radius: 8px; padding: 3px; gap: 2px; }
  .seg button { padding: 5px 14px; background: none; font-family: var(--mono); }
  .seg button.on { background: var(--card-on); }
  .right { margin-left: auto; }
</style>
