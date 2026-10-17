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
    </div>
    <!-- 引线：图 100×295，旋钮中心 y=84，三个键 y=153/201/249；卡片位置见下方 CSS。 -->
    <svg class="leader" viewBox="0 0 40 295" width="40" height="295" aria-hidden="true">
      <path d="M40 84 L0 30 M40 84 L0 132" />
    </svg>
    <DeviceView {lights} selectedIndex={sel.index < 4 ? sel.index : null} onPick={(i) => pick(i, "press")} />
    <svg class="leader" viewBox="0 0 40 295" width="40" height="295" aria-hidden="true">
      <path d="M0 84 L40 30 M0 153 L40 96 M0 201 L40 174 M0 249 L40 252" />
    </svg>
    <div class="side right">
      <div class="knobs"><KnobLabel index={4} label={actionLabel(effectivePress(ctl(4)), null, halos)} selected={sel.index === 4} onPick={() => pick(4, "press")} /></div>
      {#each [0, 1, 2] as i}
        <GestureCard index={i} control={ctl(i)} selected={sel.index === i ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, i)} onPick={(g) => pick(i, g)} />
      {/each}
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
  .right > :global(.card:first-of-type) { margin-top: 2px; }
  .leader { display: block; }
  .leader path { fill: none; stroke: #5a5a62; stroke-width: 1; }
</style>
