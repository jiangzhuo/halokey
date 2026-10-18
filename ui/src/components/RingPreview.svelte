<script lang="ts">
  import { t } from "../lib/i18n";
  import { wedgePath, wedgeCenter } from "../lib/wedge";
  import type { Slot } from "../model/halo";
  let { slots, selected, onPick }: { slots: Slot[]; selected: number; onPick: (i: number) => void } = $props();
  // 预览只负责"选中的是哪一格"，动作名在右侧列表里；尺寸缩到 260 给下方键码面板让高度。
  const SIZE = 260, C = 130, OUTER = 124, INNER = 72;
</script>

<svg class="ring" width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`}>
  {#each slots as s, i}
    <path class="wedge" class:sel={i === selected} class:filled={!!s.action} data-index={i} d={wedgePath(slots.length, i, OUTER, INNER, C, C)} role="button" tabindex="0" onclick={() => onPick(i)} onkeydown={(e) => e.key === "Enter" && onPick(i)} />
  {/each}
  {#each slots as s, i}
    {@const p = wedgeCenter(slots.length, i, (OUTER + INNER) / 2, C, C)}
    <text class="name" class:sel={i === selected} class:empty={!s.action} x={p.x} y={p.y + 4} text-anchor="middle" font-size={s.action ? 11 : 12}>{s.action ? s.label || t("（未命名）") : i + 1}</text>
  {/each}
  <circle cx={C} cy={C} r="40" class="hub" />
  <text x={C} y={C + 4} text-anchor="middle" font-size="12" class="hubtext">{selected + 1} / {slots.length}</text>
</svg>

<style>
  .wedge { fill: var(--card); stroke: var(--bg); stroke-width: 2; cursor: pointer; }
  .wedge.filled { fill: var(--card-on); }
  .wedge.sel { fill: var(--accent); }
  .name { fill: var(--text); pointer-events: none; }
  .name.empty { fill: var(--muted); font-family: var(--mono); }
  .name.sel { fill: var(--accent-text); }
  .hub { fill: var(--bg); stroke: var(--line); stroke-width: 2; }
  .hubtext { fill: var(--muted); }
</style>
