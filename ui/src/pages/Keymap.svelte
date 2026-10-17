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
      <div class="knobs"><KnobLabel index={5} label={actionLabel(effectivePress(ctl(5)), null, halos)} selected={sel.index === 5} onPick={() => pick(5, "press")} /></div>
      <div class="name">⟳ {t(CONTROL_NAMES[3])}</div>
      <GestureCard index={3} control={ctl(3)} selected={sel.index === 3 ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, 3)} onPick={(g) => pick(3, g)} />
    </div>
    <DeviceView {lights} selectedIndex={sel.index < 4 ? sel.index : null} onPick={(i) => pick(i, "press")} />
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
  .stage { display: grid; grid-template-columns: 1fr 150px 1fr; gap: 18px; padding: 4px 40px 18px; align-items: start; }
  .side { display: flex; flex-direction: column; gap: 12px; }
  /* 两个旋转标签在同一水平线上，对齐到图片里旋钮的高度（图高 377px，旋钮中心在 28.4%）。 */
  .knobs { display: flex; margin-top: 85px; }
  .left .knobs { justify-content: flex-end; }
  .right .knobs { justify-content: flex-start; }
  .name { color: var(--muted); font-size: 14px; margin-top: 14px; }
  .right > :global(.card:first-of-type) { margin-top: 14px; }
</style>
