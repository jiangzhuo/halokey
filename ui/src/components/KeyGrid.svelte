<script lang="ts">
  import type { Keycap } from "../model/codes";
  import type { KeyEntry } from "../model/keymap";
  let { keycaps, current, onPick }: { keycaps: Keycap[]; current: KeyEntry[] | null; onPick: (entries: KeyEntry[]) => void } = $props();
  const isOn = (k: Keycap) => !!current && current.length === 1 && current[0]!.type === k.entry.type && current[0]!.code === k.entry.code;
</script>
<div class="grid">
  {#each keycaps as k}<button class:on={isOn(k)} class:long={k.label.length > 8} onclick={() => onPick([k.entry])}>{k.label}</button>{/each}
</div>
<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(78px, 1fr)); gap: 8px; }
  .grid button { height: 44px; background: var(--card); font-size: 13px; line-height: 1.15; padding: 4px; }
  .grid button.on { background: var(--card-on); color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
  .grid button.long { font-size: 12px; }
</style>
