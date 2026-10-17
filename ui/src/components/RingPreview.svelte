<script lang="ts">
  import { t } from "../lib/i18n";
  import { wedgePath, wedgeCenter } from "../lib/wedge";
  import { actionLabel } from "../model/labels";
  import type { Slot } from "../model/halo";
  import type { Halo } from "../model/keymap";
  let { slots, selected, onPick, halos = [] }: { slots: Slot[]; selected: number; onPick: (i: number) => void; halos?: Halo[] } = $props();
  const SIZE = 400, C = 200, OUTER = 190, INNER = 110;
</script>

<svg class="ring" width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`}>
  {#each slots as s, i}
    <path class="wedge" class:sel={i === selected} class:filled={!!s.action} data-index={i} d={wedgePath(slots.length, i, OUTER, INNER, C, C)} role="button" tabindex="0" onclick={() => onPick(i)} onkeydown={(e) => e.key === "Enter" && onPick(i)} />
  {/each}
  {#each slots as s, i}
    {@const p = wedgeCenter(slots.length, i, (OUTER + INNER) / 2, C, C)}
    <text class="name" class:sel={i === selected} x={p.x} y={s.action ? p.y - 7 : p.y + 5} text-anchor="middle" font-size={s.action ? 12 : 18}>{s.action ? s.label || t("（未命名）") : "＋"}</text>
    {#if s.action}<text class="act" class:sel={i === selected} x={p.x} y={p.y + 9} text-anchor="middle" font-size="10">{actionLabel(s.action, null, halos)}</text>{/if}
  {/each}
  <circle cx={C} cy={C} r="58" class="hub" />
  <text x={C} y={C + 4} text-anchor="middle" font-size="13" class="hubtext">{selected + 1} / {slots.length}</text>
</svg>

<style>
  .wedge { fill: var(--card); stroke: var(--bg); stroke-width: 2; cursor: pointer; }
  .wedge.filled { fill: var(--card-on); }
  .wedge.sel { fill: var(--accent); }
  .name { fill: var(--text); pointer-events: none; }
  .act { fill: var(--muted); pointer-events: none; }
  .name.sel, .act.sel { fill: var(--accent-text); }
  .hub { fill: var(--bg); stroke: var(--line); stroke-width: 2; }
  .hubtext { fill: var(--muted); }
</style>
