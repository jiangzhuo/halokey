<script lang="ts">
  import { onMount } from "svelte";
  import { t, colon, listSep, sentenceSep } from "../lib/i18n";
  import { api, type WindowInfo } from "../lib/tauri";
  import { newUuid, nextSlot, upsertLibraryItem, deleteLibraryItem, type Draft } from "../model/draft";
  import type { ApplicationTarget } from "../model/keymap";

  /** 添加 / 编辑功能库里的应用：名称、WM_CLASS、可执行路径，可从当前窗口列表挑。 */
  let { draft = $bindable(), itemId, onClose }: { draft: Draft; itemId: string | null; onClose: (savedId: string | null) => void } = $props();
  // svelte-ignore state_referenced_locally
  const existing = itemId ? (draft.map?.actionLibrary ?? []).find((i) => i.id === itemId) ?? null : null;
  const target: ApplicationTarget | null = existing && "application" in existing.action ? (existing.action as { application: { _0: ApplicationTarget } }).application._0 : null;
  let name = $state(existing?.name ?? "");
  let cls = $state(target?.bundleIdentifier ?? "");
  let path = $state(target?.path ?? "");
  let windows = $state<WindowInfo[]>([]);
  let listError = $state("");
  let problems = $state<string[]>([]);
  let blocked = $state<string[]>([]);
  onMount(async () => {
    try { const r = await api.listWindows(); windows = r.windows; listError = r.error ?? ""; }
    catch (e) { listError = String(e); }
  });

  function pick(w: WindowInfo) {
    cls = w.class;
    if (!path) path = w.exe;
    // 窗口标题常常很长；功能库项的名称 Rust 限 80 字。
    if (!name.trim()) name = [...(w.title || w.class)].slice(0, 80).join("");
  }
  function save() {
    const p: string[] = [];
    if (!name.trim()) p.push(t("名称不能为空"));
    else if ([...name.trim()].length > 80) p.push(t("名称不能超过 80 字"));
    if (!cls.trim() && !path.trim()) p.push(t("WM_CLASS 和路径至少填一个"));
    problems = p;
    if (p.length) return;
    const id = existing?.id ?? newUuid();
    const slot = existing?.slot ?? nextSlot(draft.map!, "application");
    if (slot === null) { problems = [t("应用最多 16 个")]; return; }
    draft = upsertLibraryItem(draft, { id, name: name.trim(), slot, action: { application: { _0: { bundleIdentifier: cls.trim(), path: path.trim(), name: name.trim() } } } });
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

<div class="backdrop" role="presentation" onmousedown={(e) => { if (e.target === e.currentTarget) onClose(null); }}>
  <div class="dialog" role="dialog" aria-modal="true">
    <h3>{existing ? t("编辑 APP") : t("添加 APP")}</h3>
    <label><span>{t("名称")}</span><input data-role="app-name" bind:value={name} placeholder={t("显示名称")} /></label>
    <label><span>WM_CLASS</span><input data-role="app-class" bind:value={cls} placeholder="google-chrome" /></label>
    <label><span>{t("可执行文件路径")}</span><input data-role="app-path" bind:value={path} placeholder="/usr/bin/google-chrome" /></label>
    <div class="windows">
      <div class="muted">{t("从当前打开的窗口选")}{#if listError}{colon()}{listError}{/if}</div>
      <div class="list">
        {#each windows as w (w.class)}<button data-window={w.class} onclick={() => pick(w)}><b>{w.class}</b><small>{w.title}</small></button>{/each}
        {#if !windows.length && !listError}<span class="muted">{t("没有找到窗口")}</span>{/if}
      </div>
    </div>
    {#if problems.length}<p class="bad">{problems.join(sentenceSep())}</p>{/if}
    {#if blocked.length}<p class="bad">{t("被以下位置引用，无法删除")}{colon()}{blocked.join(listSep())}</p>{/if}
    <footer>
      {#if existing}<button class="danger" onclick={remove}>{t("删除")}</button>{/if}
      <button class="ghost" onclick={() => onClose(null)}>{t("取消")}</button>
      <button class="primary" onclick={save}>{t("完成")}</button>
    </footer>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: var(--scrim); display: flex; align-items: center; justify-content: center; z-index: 50; }
  .dialog { width: 560px; max-width: calc(100vw - 32px); background: var(--surface); border: 1px solid var(--line); border-radius: var(--r-lg); padding: 18px 20px; display: flex; flex-direction: column; gap: 10px; box-shadow: var(--shadow-modal); }
  h3 { margin: 0 0 4px; font-size: 16px; }
  label { display: grid; grid-template-columns: 120px 1fr; align-items: center; gap: 10px; font-size: 13px; color: var(--muted); }
  label input { width: 100%; }
  .windows { border-top: 1px solid var(--line); padding-top: 10px; }
  .list { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; max-height: 180px; overflow: auto; }
  .list button { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; min-width: 150px; max-width: 240px; }
  .list small { color: var(--muted); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px; }
  .muted { color: var(--muted); font-size: 13px; }
  .bad { color: var(--bad); font-size: 13px; margin: 0; }
  footer { display: flex; justify-content: flex-end; gap: 10px; margin-top: 6px; }
</style>
