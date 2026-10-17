<script lang="ts">
  // 实物渲染图来自 Open VibeKey（MIT）的 landing/device.png，328×968；热区与灯圈按百分比叠在图上。
  let { lights, selectedIndex, onPick }: { lights: ("off" | "solid" | "breathe")[]; selectedIndex: number | null; onPick: (index: number) => void } = $props();
  const ring = (i: number) => lights[i] ?? "off";
  // [index, 中心 x%, 中心 y%, 直径占宽度的百分比]
  const SPOTS: [number, number, number, number][] = [[3, 49.7, 28.4, 70], [0, 49.7, 51.9, 37], [1, 49.7, 68.2, 37], [2, 49.7, 84.5, 37]];
  const pct = (v: number) => `${v}%`;
</script>

<div class="device" role="img" aria-label="Vibe Key">
  <img src="/device.png" alt="" draggable="false" />
  {#each SPOTS as [i, x, y, d]}
    <button class="spot" class:sel={selectedIndex === i} data-mode={ring(i)} style:left={pct(x - d / 2)} style:top={pct(y - d / 2 * 328 / 968)} style:width={pct(d)} style:height={pct(d * 328 / 968)} onclick={() => onPick(i)} aria-label={`control ${i}`}></button>
  {/each}
</div>

<style>
  .device { position: relative; width: 100px; margin: 0 auto; } /* 高 295px；舞台布局按这个高度算 */
  .device img { display: block; width: 100%; height: auto; user-select: none; }
  .spot { position: absolute; padding: 0; background: transparent; border-radius: 50%; border: 3px solid transparent; cursor: pointer; }
  .spot:hover { border-color: rgba(242, 201, 181, .45); filter: none; }
  .spot[data-mode="solid"] { box-shadow: 0 0 0 3px var(--accent), 0 0 14px 3px rgba(242, 201, 181, .55); }
  .spot[data-mode="breathe"] { box-shadow: 0 0 0 3px var(--accent), 0 0 14px 3px rgba(242, 201, 181, .55); animation: breathe 1.8s ease-in-out infinite; }
  .spot.sel { border-color: var(--accent); }
  @keyframes breathe { 0%, 100% { opacity: .3; } 50% { opacity: 1; } }
</style>
