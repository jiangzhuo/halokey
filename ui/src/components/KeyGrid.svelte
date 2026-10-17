<script lang="ts">
  import type { Keycap } from "../model/codes";
  import type { KeyEntry } from "../model/keymap";
  let { keycaps, current, showTransparent, onPick, onClear, onTransparent }: { keycaps: Keycap[]; current: KeyEntry[] | null; showTransparent: boolean; onPick: (entries: KeyEntry[]) => void; onClear: () => void; onTransparent: () => void } = $props();
  const isOn = (k: Keycap) => !!current && current.length === 1 && current[0]!.type === k.entry.type && current[0]!.code === k.entry.code;
</script>
<div class="grid">
  {#if showTransparent}<button class="special" title="透明继承" onclick={onTransparent}>▽</button>{/if}
  <button class="special" title="不执行动作 / 关闭手势" onclick={onClear}>⊠</button>
  {#each keycaps as k}<button class:on={isOn(k)} onclick={() => onPick([k.entry])}>{k.label}</button>{/each}
</div>
<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(78px, 1fr)); gap: 8px; }
  .grid button { height: 44px; background: var(--card); font-size: 13px; line-height: 1.15; padding: 4px; }
  .grid button.on { background: var(--accent); color: var(--accent-text); }
  .grid .special { color: var(--muted); }
</style>
