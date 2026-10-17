<script lang="ts">
  import { t } from "../lib/i18n";
  import KeyGrid from "./KeyGrid.svelte";
  import OutputModeSegment from "./OutputModeSegment.svelte";
  import { CATEGORIES, type CategoryId } from "../model/codes";
  import { CONTROL_NAMES } from "../model/labels";
  import { isRotation, type Behavior, type Halo, type KeyEntry, type NamedHostAction } from "../model/keymap";
  import { assignAction, assignEntries, clearGesture, resolvedControl, setBehavior, setTapCount, setTransparent, type Draft } from "../model/draft";

  let { draft = $bindable(), halos, apps, macros }: { draft: Draft; halos: Halo[]; apps: NamedHostAction[]; macros: NamedHostAction[] } = $props();
  let category = $state<CategoryId>("common");
  let step = $state(20);
  const sel = $derived(draft.selection);
  const control = $derived(draft.map ? resolvedControl(draft.map, sel.layer, sel.index) : null);
  const gestureName: Record<string, string> = { press: "单击", double: "双击", long: "长按" };
  const current = $derived<KeyEntry[] | null>(!control ? null : sel.gesture === "press" ? (control.pressAction ? null : control.press) : sel.gesture === "double" ? (control.doublePressAction ? null : control.doublePress ?? null) : (control.longPressAction ? null : control.longPress ?? null));
  const behavior = $derived<Behavior>(!control ? "hold" : sel.gesture === "press" ? control.pressBehavior ?? "hold" : sel.gesture === "double" ? control.doublePressBehavior ?? "tap" : control.longPressBehavior ?? "hold");
  const count = $derived(!control ? 2 : sel.gesture === "press" ? control.pressTapCount ?? 2 : sel.gesture === "double" ? control.doublePressTapCount ?? 2 : control.longPressTapCount ?? 2);
  const rotation = $derived(isRotation(sel.index));
  const cat = $derived(CATEGORIES.find((c) => c.id === category)!);
  const layerAllowed = $derived(!rotation && sel.gesture !== "double");
  const pointer = (dx: number, dy: number) => { draft = assignAction(draft, { pointerMove: { _0: { dx, dy } } }); };
</script>

<section class="panel">
  <nav class="cats">
    {#each CATEGORIES as c, i}
      {#if i === 5}<hr>{/if}
      <button class:on={category === c.id} disabled={c.id === "layer" && !layerAllowed} onclick={() => (category = c.id)}>{t(c.name)}</button>
    {/each}
  </nav>
  <div class="pal">
    <div class="head">
      <span class="crumb">{t(CONTROL_NAMES[sel.index] ?? "")} › {t(gestureName[sel.gesture] ?? "")}</span>
      <div class="mode"><OutputModeSegment {behavior} {count} allowHold={!rotation && sel.gesture !== "double"} onBehavior={(b) => (draft = setBehavior(draft, b))} onCount={(n) => (draft = setTapCount(draft, n))} /></div>
    </div>
    {#if cat.keycaps}
      <KeyGrid keycaps={cat.keycaps} {current} showTransparent={sel.layer !== 0} onPick={(e) => (draft = assignEntries(draft, e))} onClear={() => (draft = clearGesture(draft))} onTransparent={() => (draft = setTransparent(draft))} />
    {:else if category === "layer"}
      <div class="cards">
        {#if sel.layer !== 0}<button class="special" onclick={() => (draft = setTransparent(draft))}>▽</button>{/if}
        <button class="special" onclick={() => (draft = clearGesture(draft))}>⊠</button>
        {#each [1, 2, 3] as l}<button onclick={() => (draft = assignAction(draft, { momentaryLayer: { _0: l } }))}>MO({l})</button>{/each}
      </div>
    {:else if category === "pointer"}
      <div class="cards">
        <button onclick={() => pointer(0, -step)}>↑</button><button onclick={() => pointer(0, step)}>↓</button>
        <button onclick={() => pointer(-step, 0)}>←</button><button onclick={() => pointer(step, 0)}>→</button>
        <label>{t("指针步长")} <input type="number" min="1" max="1000" bind:value={step}> {t("像素")}</label>
      </div>
    {:else if category === "halo"}
      <div class="cards">
        {#each halos as h, i}<button class="item" onclick={() => (draft = assignAction(draft, { halo: { _0: h.id } }))}><small>◎ H{i}</small>{h.name}</button>{/each}
        {#if !halos.length}<span class="muted">{t("在本机构建里还没有的功能")}</span>{/if}
      </div>
    {:else if category === "app" || category === "macro"}
      <div class="cards">
        {#each (category === "app" ? apps : macros) as item}<button class="item" onclick={() => (draft = assignAction(draft, { library: { _0: item.id } }))}><small>{category === "app" ? "A" : "M"}{item.slot}</small>{item.name}</button>{/each}
        <span class="muted">{t("在本机构建里还没有的功能")}</span>
      </div>
    {:else}
      <span class="muted">{t("在本机构建里还没有的功能")}</span>
    {/if}
  </div>
</section>

<style>
  .panel { display: grid; grid-template-columns: 150px 1fr; border-top: 1px solid var(--line); min-height: 280px; }
  .cats { padding: 14px 12px; display: flex; flex-direction: column; gap: 2px; border-right: 1px solid var(--line); }
  .cats button { text-align: left; background: none; padding: 7px 10px; color: #c9c9ce; }
  .cats button.on { background: var(--card-on); color: var(--text); }
  .cats hr { border: 0; border-top: 1px solid var(--line); margin: 8px 0; width: 100%; }
  .pal { padding: 14px 18px; }
  .head { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
  .crumb { font-size: 15px; }
  .mode { margin-left: auto; }
  .cards { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
  .cards button { min-width: 86px; height: 44px; }
  .cards .item { display: flex; flex-direction: column; align-items: center; gap: 2px; width: 150px; height: 60px; }
  .cards .item small { color: var(--muted); font-family: var(--mono); font-size: 11px; }
  .cards .special { color: var(--muted); }
  .muted { color: var(--muted); font-size: 13px; }
</style>
