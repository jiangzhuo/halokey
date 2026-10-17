<script lang="ts">
  import LayerBar from "../components/LayerBar.svelte";
  import DeviceView from "../components/DeviceView.svelte";
  import GestureCard from "../components/GestureCard.svelte";
  import KnobLabel from "../components/KnobLabel.svelte";
  import Palette from "../components/Palette.svelte";
  import BottomBar from "../components/BottomBar.svelte";
  import { api } from "../lib/tauri";
  import { markSaved, revert } from "../model/draft";
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  import { actionLabel, CONTROL_NAMES } from "../model/labels";
  import { effectivePress, type Gesture } from "../model/keymap";
  import { controlAt, resolvedControl, select, type Draft } from "../model/draft";

  let { draft = $bindable(), snapshot }: { draft: Draft; snapshot: UiSnapshot | null } = $props();
  const map = $derived(draft.map);
  const sel = $derived(draft.selection);
  const halos = $derived(map?.halos ?? []);
  const ctl = (i: number) => resolvedControl(map!, sel.layer, i);
  const pick = (index: number, gesture: Gesture) => { draft = select(draft, { layer: sel.layer, index, gesture }); };
  // 引线：从图上控件出发先水平一段，再折向卡片高度，再水平接入卡片（左右镜像）。
  const leader = (side: "left" | "right", from: number, to: number) =>
    side === "left" ? `M40 ${from} H30 L10 ${to} H0` : `M0 ${from} H10 L30 ${to} H40`;
  const LEFT: [number, number, number][] = [[5, 84, 30], [3, 84, 127], [1, 201, 211]];
  const RIGHT: [number, number, number][] = [[4, 84, 30], [0, 153, 127], [2, 249, 211]];
  const lights = $derived(["off", "off", "off", "off"] as ("off" | "solid" | "breathe")[]); // 第 2 期接本层背光
  const _snapshot = $derived(snapshot); // 第 2 期接本层背光时使用
  let saving = $state(false);
  let message = $state("");
  let error = $state(false);
  async function save() {
    if (!draft.map) return;
    saving = true; error = false; message = "";
    try { await api.validateKeymap(draft.map); await api.saveKeymap(draft.map); draft = markSaved(draft); message = t("已保存"); }
    catch (e) { error = true; message = String(e); }
    finally { saving = false; }
  }
</script>

{#if !map}
  <p class="empty">{t("未连接")}</p>
{:else}
  <LayerBar layer={sel.layer} onSelect={(l) => { draft = select(draft, { ...sel, layer: l }); }} />
  <section class="stage">
    <div class="side left">
      <div class="knobs end"><KnobLabel index={5} label={actionLabel(effectivePress(ctl(5)), null, halos)} selected={sel.index === 5} onPick={() => pick(5, "press")} /></div>
      <div class="name">⟳ {t(CONTROL_NAMES[3])}</div>
      <GestureCard index={3} control={ctl(3)} selected={sel.index === 3 ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, 3)} onPick={(g) => pick(3, g)} />
      <GestureCard index={1} control={ctl(1)} selected={sel.index === 1 ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, 1)} onPick={(g) => pick(1, g)} />
    </div>
    <svg class="leader" viewBox="0 0 40 295" width="40" height="295" aria-hidden="true">
      {#each LEFT as [i, from, to]}
        <path d={leader("left", from, to)} class:sel={sel.index === i} />
        <circle cx="40" cy={from} r="2.5" class:sel={sel.index === i} />
      {/each}
    </svg>
    <DeviceView {lights} selectedIndex={sel.index < 4 ? sel.index : null} onPick={(i) => pick(i, "press")} />
    <svg class="leader" viewBox="0 0 40 295" width="40" height="295" aria-hidden="true">
      {#each RIGHT as [i, from, to]}
        <path d={leader("right", from, to)} class:sel={sel.index === i} />
        <circle cx="0" cy={from} r="2.5" class:sel={sel.index === i} />
      {/each}
    </svg>
    <div class="side right">
      <div class="knobs"><KnobLabel index={4} label={actionLabel(effectivePress(ctl(4)), null, halos)} selected={sel.index === 4} onPick={() => pick(4, "press")} /></div>
      <GestureCard index={0} control={ctl(0)} selected={sel.index === 0 ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, 0)} onPick={(g) => pick(0, g)} />
      <GestureCard index={2} control={ctl(2)} selected={sel.index === 2 ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, 2)} onPick={(g) => pick(2, g)} />
    </div>
  </section>
  <Palette bind:draft {halos} apps={(map.actionLibrary ?? []).filter((a) => "application" in a.action)} macros={(map.actionLibrary ?? []).filter((a) => "macro" in a.action)} />
  <BottomBar dirty={draft.dirty} {saving} {message} {error} onSave={save} onRevert={() => { draft = revert(draft); message = ""; }} />
{/if}

<style>
  .empty { padding: 40px; color: var(--muted); text-align: center; }
  /* 参考 olanzi：渲染图 100×295，舞台高度就是图高。左右旋在最上面一行（高于旋钮），旋钮按下在其下；
     右侧三张卡片紧凑堆叠；位置对不上的用引线指到图上的控件。 */
  .stage { display: grid; grid-template-columns: 1fr 40px 100px 40px 1fr; align-items: start; padding: 4px 40px 14px; }
  .side { display: flex; flex-direction: column; gap: 6px; height: 295px; }
  .knobs { display: flex; margin-top: 8px; }
  .knobs.end { justify-content: flex-end; }
  .left .name { color: var(--muted); font-size: 14px; margin-top: 12px; }
  .left > :global(.card:last-of-type) { margin-top: 6px; }
  /* 右侧两张卡片与左侧两张同高（顶 91 / 175 px），左右对称。 */
  .right > :global(.card:nth-of-type(2)) { margin-top: 39px; } /* 第一个 div 是旋转标签行，卡片从第二个起 */
  .leader { display: block; overflow: visible; }
  .leader path { fill: none; stroke: #55555d; stroke-width: 1.5; stroke-linejoin: round; stroke-linecap: round; transition: stroke 120ms; }
  .leader circle { fill: #55555d; transition: fill 120ms; }
  .leader path.sel { stroke: var(--accent); }
  .leader circle.sel { fill: var(--accent); }
</style>
