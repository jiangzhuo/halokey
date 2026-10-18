<script lang="ts">
  import type { Keycap } from "../model/codes";
  import type { KeyEntry } from "../model/keymap";
  import { t } from "../lib/i18n";
  let { keycaps, current, showTransparent, showClear = true, onPick, onClear, onTransparent }: { keycaps: Keycap[]; current: KeyEntry[] | null; showTransparent: boolean; showClear?: boolean; onPick: (entries: KeyEntry[]) => void; onClear: () => void; onTransparent: () => void } = $props();
  const isOn = (k: Keycap) => !!current && current.length === 1 && current[0]!.type === k.entry.type && current[0]!.code === k.entry.code;
</script>
<div class="grid">
  {#if showTransparent}<button class="special" title={t("透明继承")} onclick={onTransparent}>▽</button>{/if}
  {#if showClear}<button class="special" title={`${t("不执行动作")} / ${t("关闭手势")}`} onclick={onClear}>⊠</button>{/if}
  {#each keycaps as k}<button class:on={isOn(k)} class:long={k.label.length > 8} onclick={() => onPick([k.entry])}>{k.label}</button>{/each}
</div>
<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(78px, 1fr)); gap: 8px; }
  .grid button { height: 44px; background: var(--card); font-size: 13px; line-height: 1.15; padding: 4px; }
  .grid button.on { background: var(--card-on); color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
  .grid button.long { font-size: 12px; }
  .grid .special { color: var(--muted); background: transparent; box-shadow: inset 0 0 0 1px var(--line-strong); }
</style>
