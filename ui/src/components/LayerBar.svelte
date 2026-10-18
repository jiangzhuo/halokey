<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../lib/i18n";
  let { layer, onSelect, right, active = [] }: { layer: number; onSelect: (l: number) => void; right?: Snippet; active?: number[] } = $props();
</script>
<div class="bar">
  <span class="muted">Layer</span>
  <div class="seg">{#each [0, 1, 2, 3] as l}<button class:on={layer === l} class:held={active.includes(l)} onclick={() => onSelect(l)}>{l}{#if active.includes(l)}<i class="live" title={t("设备当前按住的层")}></i>{/if}</button>{/each}</div>
  <div class="right">{@render right?.()}</div>
</div>
<style>
  .bar { display: flex; align-items: flex-start; gap: 14px; padding: 12px 22px; }
  .bar > .muted, .bar > .seg { margin-top: 4px; }
  .muted { color: var(--muted); font-size: 13px; }
  .seg { display: inline-flex; background: var(--card); border-radius: var(--r-md); padding: 3px; gap: 2px; }
  .seg button { padding: 5px 14px; background: none; font-family: var(--mono); position: relative; }
  .seg button:hover { background: var(--card-on); }
  .seg button.on { background: var(--card-on); color: var(--accent); }
  /* 设备正按住的层：数字变绿，右上角一个小点；绝对定位，不改变按钮宽度。 */
  .seg button.held { color: var(--ok); }
  .live { position: absolute; top: 3px; right: 4px; width: 5px; height: 5px; border-radius: 50%; background: var(--ok); }
  .right { margin-left: auto; }
</style>
