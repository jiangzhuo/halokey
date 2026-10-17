<script lang="ts">
  import { t } from "../lib/i18n";
  import { wedgePath, wedgeCenter } from "../lib/wedge";
  import type { Halo } from "../model/keymap";
  let { halo, selected, size }: { halo: Halo | null; selected: number; size: number } = $props();
  const outer = $derived(size / 2 - 4);
  const inner = $derived(outer * 0.6);
  const c = $derived(size / 2);
  const n = $derived(halo?.options.length ?? 0);
  const labelR = $derived((outer + inner) / 2);
  const fontSize = $derived(n > 6 ? 12 : 13);
</script>

{#if halo}
  <div class="wheel" style:width={`${size}px`}>
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      {#each halo.options as _o, i}
        <path class="wedge" class:sel={i === selected} data-index={i} d={wedgePath(n, i, outer, inner, c, c)} />
      {/each}
      {#each halo.options as o, i}
        {@const p = wedgeCenter(n, i, labelR, c, c)}
        <text class="label" class:sel={i === selected} x={p.x} y={p.y} font-size={fontSize} text-anchor="middle" dominant-baseline="middle">{o.label}</text>
      {/each}
    </svg>
    <div class="center" style:top={`${c}px`} style:width={`${inner * 1.7}px`} style:height={`${inner * 1.7}px`}>{halo.options[selected]?.label ?? halo.name}</div>
    <div class="hint">{t("拧旋钮选择 · 按下确认 · 按任意键取消")}</div>
  </div>
{/if}

<style>
  :global(html, body) { margin: 0; background: transparent; overflow: hidden; }
  .wheel { position: relative; font: 13px/1.3 system-ui, sans-serif; color: #e6e9f0; }
  .wedge { fill: rgba(27, 31, 42, .9); stroke: rgba(230, 233, 240, .18); stroke-width: 1.5; transition: fill 90ms; }
  .wedge.sel { fill: #f2c9b5; }
  .label { fill: #e6e9f0; pointer-events: none; }
  .label.sel { fill: #0b1220; font-weight: 700; }
  .center { position: absolute; left: 50%; transform: translate(-50%, -50%); border-radius: 50%; background: rgba(17, 20, 28, .94); border: 2px solid rgba(242, 201, 181, .6); display: flex; align-items: center; justify-content: center; text-align: center; font-weight: 600; padding: 8px; box-sizing: border-box; }
  .hint { text-align: center; color: rgba(230, 233, 240, .6); font-size: 11px; height: 28px; line-height: 28px; }
</style>
