<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";
  import { api, type WindowInfo } from "../lib/tauri";
  import ComboRecorder from "../components/ComboRecorder.svelte";
  import { stepLabel, stepKind, macroProblems, moveStep, removeStep, insertStep } from "../model/macro";
  import { newUuid, nextSlot, upsertLibraryItem, deleteLibraryItem, libraryItems, type Draft } from "../model/draft";
  import type { MacroStep, NamedHostAction, ApplicationTarget, KeyEntry } from "../model/keymap";

  /** 新建 / 编辑功能库里的宏：步骤列表在左，右侧添加组合键 / APP / 等待。保存时一次写回草稿。 */
  let { draft = $bindable(), itemId, onClose }: { draft: Draft; itemId: string | null; onClose: (savedId: string | null) => void } = $props();
  // svelte-ignore state_referenced_locally
  const existing = itemId ? (draft.map?.actionLibrary ?? []).find((i) => i.id === itemId) ?? null : null;
  const initialSteps: MacroStep[] = existing && "macro" in existing.action ? JSON.parse(JSON.stringify((existing.action as { macro: { _0: MacroStep[] } }).macro._0)) : [];
  let name = $state(existing?.name ?? "");
  let steps = $state<MacroStep[]>(initialSteps);
  let selected = $state<number | null>(null);
  let delay = $state(0.3);
  let windows = $state<WindowInfo[]>([]);
  let listError = $state("");
  let problems = $state<string[]>([]);
  let blocked = $state<string[]>([]);
  const apps = $derived(draft.map ? libraryItems(draft.map, "application") : []);
  const ICON: Record<string, string> = { keyboard: "⌨", application: "▣", delay: "⏱" };
  onMount(async () => {
    try { const r = await api.listWindows(); windows = r.windows; listError = r.error ?? ""; }
    catch (e) { listError = String(e); }
  });

  /** 新步骤插在选中步骤之后（没有选中就放末尾），并选中它。 */
  function add(step: MacroStep) {
    const at = selected === null ? steps.length : selected + 1;
    steps = insertStep(steps, at, step);
    selected = at;
    problems = [];
  }
  // 宏步骤不能引用功能库，所以把应用内容复制进来。
  const addApp = (a: NamedHostAction) => add({ application: { _0: JSON.parse(JSON.stringify((a.action as { application: { _0: ApplicationTarget } }).application._0)) } });
  const addWindow = (w: WindowInfo) => add({ application: { _0: { bundleIdentifier: w.class, path: w.exe, name: [...(w.title || w.class)].slice(0, 200).join("") } } });
  const addKeys = (entries: KeyEntry[]) => add({ keyboard: { _0: entries } });
  function pop() {
    if (!steps.length) return;
    steps = removeStep(steps, steps.length - 1);
    selected = steps.length ? Math.min(selected ?? 0, steps.length - 1) : null;
  }
  function save() {
    problems = macroProblems(name, steps);
    if (problems.length) return;
    const id = existing?.id ?? newUuid();
    const slot = existing?.slot ?? nextSlot(draft.map!, "macro");
    if (slot === null) { problems = [t("宏最多 16 个")]; return; }
    draft = upsertLibraryItem(draft, { id, name: name.trim(), slot, action: { macro: { _0: steps } } });
    onClose(id);
  }
  function remove() {
    if (!existing) return;
    const r = deleteLibraryItem(draft, existing.id);
    if (r.blockedBy.length) { blocked = r.blockedBy; return; }
    draft = r.draft;
    onClose(null);
  }
</script>

<div class="page">
  <header>
    <button class="back" onclick={() => onClose(null)} title={t("返回")}>←</button>
    <h2>{existing ? t("编辑宏") : t("新建宏")}</h2>
    <input data-role="macro-name" placeholder={t("宏名称")} bind:value={name} />
  </header>
  <section class="body">
    <div class="steps">
      <div class="muted head">{t("步骤")}（{steps.length}/32）<button class="small" data-role="pop-step" disabled={!steps.length} onclick={pop}>{t("清空最后一步")}</button></div>
      <ol>
        {#each steps as s, i}
          <li data-step={i} class:on={i === selected}>
            <button class="row" onclick={() => (selected = i)}><span class="n">{i + 1}</span><span class="ic">{ICON[stepKind(s)]}</span><span class="lbl">{stepLabel(s)}</span></button>
            <span class="ops">
              <button data-role="step-up" disabled={i === 0} onclick={() => { steps = moveStep(steps, i, -1); selected = i - 1; }}>↑</button>
              <button data-role="step-down" disabled={i === steps.length - 1} onclick={() => { steps = moveStep(steps, i, 1); selected = i + 1; }}>↓</button>
              <button data-role="step-remove" onclick={() => { steps = removeStep(steps, i); selected = null; }}>✕</button>
            </span>
          </li>
        {/each}
      </ol>
      {#if !steps.length}<p class="muted">{t("右边添加步骤；新步骤插在选中步骤之后")}</p>{/if}
    </div>
    <div class="adders">
      <h4>⌨ {t("组合键")}</h4>
      <p class="muted warn">{t("录制时 halokey 窗口有焦点，设备注入的按键也会被录进来")}</p>
      <ComboRecorder assignLabel={t("添加为步骤")} onAssign={addKeys} />
      <h4>▣ APP</h4>
      <div class="cards">
        {#each apps as a (a.id)}<button data-add-app={a.id} onclick={() => addApp(a)}><b>A{a.slot} {a.name}</b></button>{/each}
        {#each windows as w (w.class)}<button data-add-window={w.class} onclick={() => addWindow(w)}><b>{w.class}</b><small>{w.title}</small></button>{/each}
        {#if listError}<span class="muted">{listError}</span>{/if}
      </div>
      <h4>⏱ {t("等待")}</h4>
      <div class="delay"><input type="number" data-role="delay-seconds" min="0.05" max="10" step="0.05" bind:value={delay} /> {t("秒")} <button data-role="add-delay" onclick={() => add({ delay: { _0: Number(delay) } })}>{t("添加等待")}</button></div>
    </div>
  </section>
  <footer>
    {#if problems.length}<span class="bad">{problems.join("；")}</span>{/if}
    {#if blocked.length}<span class="bad">{t("被以下位置引用，无法删除")}：{blocked.join("、")}</span>{/if}
    {#if existing}<button class="danger" onclick={remove}>{t("删除宏")}</button>{/if}
    <button class="ghost" onclick={() => onClose(null)}>{t("取消")}</button>
    <button class="primary" onclick={save}>{t("保存宏")}</button>
  </footer>
</div>

<style>
  .page { display: flex; flex-direction: column; min-height: 100%; }
  header { display: flex; align-items: center; gap: 14px; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  header h2 { margin: 0; font-size: 16px; }
  header input { flex: 0 0 260px; }
  .back { background: none; font-size: 18px; padding: 2px 8px; }
  .back:hover { background: var(--card); }
  .body { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; padding: 16px 22px; align-items: start; flex: 1; }
  .head { display: flex; align-items: center; }
  ol { list-style: none; margin: 8px 0 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  li { display: flex; align-items: center; gap: 6px; background: var(--card); border-radius: var(--r-lg); padding: 4px 8px 4px 4px; border: 1px solid transparent; }
  li.on { border-color: var(--accent); }
  .row { flex: 1; min-width: 0; display: flex; align-items: center; gap: 10px; background: none; text-align: left; }
  .n { color: var(--muted); font-family: var(--mono); width: 22px; }
  .ic { width: 20px; text-align: center; color: var(--muted); }
  .lbl { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ops { display: flex; gap: 4px; }
  .ops button { padding: 2px 8px; }
  .adders h4 { margin: 14px 0 6px; font-size: 13px; color: var(--muted); }
  .adders h4:first-child { margin-top: 0; }
  .cards { display: flex; flex-wrap: wrap; gap: 8px; }
  .cards button { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; max-width: 240px; }
  .cards small { color: var(--muted); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px; }
  .delay { display: flex; align-items: center; gap: 8px; }
  .delay input { width: 90px; }
  .small { padding: 2px 8px; font-size: 12px; margin-left: 8px; }
  .muted { color: var(--muted); font-size: 13px; }
  .warn { margin: 0 0 8px; }
  footer { display: flex; justify-content: flex-end; align-items: center; gap: 10px; padding: 12px 22px; border-top: 1px solid var(--line); position: sticky; bottom: 0; background: var(--bg); }
  .bad { color: var(--bad); margin-right: auto; font-size: 13px; }
</style>
