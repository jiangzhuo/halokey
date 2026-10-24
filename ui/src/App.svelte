<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "./components/TopBar.svelte";
  import Keymap from "./pages/Keymap.svelte";
  import Settings from "./pages/Settings.svelte";
  import HaloEditor from "./pages/HaloEditor.svelte";
  import MacroEditor from "./pages/MacroEditor.svelte";
  import { api, subscribe, type UiSnapshot } from "./lib/tauri";
  import { t } from "./lib/i18n";
  import { initialDraft, loadSaved, assignAction, select, type Draft, type Selection } from "./model/draft";

  let page = $state<"keymap" | "settings" | "halo" | "macro">("keymap");
  // 光环 / 宏编辑器：记住从哪个手势进来的，新建成功后把新条目分配回去。
  let editing = $state<{ kind: "halo" | "macro"; id: string | null; wasNew: boolean; returnTo: Selection } | null>(null);
  function openHalo(id: string | null) {
    editing = { kind: "halo", id, wasNew: id === null, returnTo: draft.selection };
    page = "halo";
  }
  function openMacro(id: string | null) {
    editing = { kind: "macro", id, wasNew: id === null, returnTo: draft.selection };
    page = "macro";
  }
  function closeEditor(savedId: string | null) {
    if (editing?.wasNew && savedId) {
      const action = editing.kind === "halo" ? { halo: { _0: savedId } } : { library: { _0: savedId } };
      draft = assignAction(select(draft, editing.returnTo), action);
    }
    editing = null;
    page = "keymap";
  }
  let snapshot = $state<UiSnapshot | null>(null);
  let draft = $state<Draft>(initialDraft());
  let langTick = $state(0);
  let activeLayers = $state<number[]>([]);
  // 后端最近一次的错误（快照里的 lastError）。用户点掉后不再显示同一条，直到来了新的错误事件。
  let dismissedError = $state<string | null>(null);
  const shownError = $derived(snapshot?.lastError && snapshot.lastError !== dismissedError ? snapshot.lastError : null);

  async function refresh() {
    try { snapshot = await api.snapshot(); activeLayers = snapshot.activeLayers ?? []; } catch (e) { console.error(e); }
    // 键位来自配置文件，与接收器是否在线无关：离线也要能编辑。
    if (!draft.map) try { draft = loadSaved(draft, await api.getKeymap()); } catch { /* 还没有键位（等待接收器），下次再试 */ }
  }
  onMount(() => {
    refresh();
    const timer = setInterval(refresh, 2000);
    const relang = () => { langTick++; };
    window.addEventListener("halokey-lang", relang);
    let un: (() => void) | undefined;
    subscribe((e) => {
      if (e.kind === "connected") api.getKeymap().then((m) => { draft = loadSaved(draft, m); });
      if (e.kind === "layers") activeLayers = e.active;
      // 不等下一次轮询：马上显示，之前点掉过同样的文本也重新显示。
      if (e.kind === "error") { dismissedError = null; if (snapshot) snapshot = { ...snapshot, lastError: e.text }; }
    }).then((u) => { un = u; });
    return () => { clearInterval(timer); window.removeEventListener("halokey-lang", relang); un?.(); };
  });
</script>

{#key langTick}
<div class="app">
  <TopBar page={page === "settings" ? "settings" : "keymap"} {snapshot} dirty={draft.dirty} onNavigate={(p) => { editing = null; page = p; }} />
  {#if shownError}
    <div class="error" role="alert" data-role="backend-error"><span>⚠ {shownError}</span><button data-role="dismiss-error" aria-label={t("忽略")} title={t("忽略")} onclick={() => { dismissedError = shownError; }}>✕</button></div>
  {/if}
  <div class="view">
  {#if page === "keymap"}
    <Keymap bind:draft {snapshot} {activeLayers} onEditHalo={openHalo} onEditMacro={openMacro} />
  {:else if page === "halo"}
    <HaloEditor bind:draft haloId={editing?.id ?? null} onClose={closeEditor} />
  {:else if page === "macro"}
    <MacroEditor bind:draft itemId={editing?.id ?? null} onClose={closeEditor} />
  {:else}
    <Settings {snapshot} dirty={draft.dirty} hasBacklight={!!draft.saved?.lighting || (draft.saved?.layers ?? []).some((l) => !!l.lighting)} onKeymapReplaced={(m) => { draft = loadSaved(initialDraft(), m); }} />
  {/if}
  </div>
</div>
{/key}

<style>
  /* 顶栏固定；页面内容在 .view 里滚动（键位页自己再把舞台固定、只滚键码面板）。 */
  .app { display: flex; flex-direction: column; height: 100vh; }
  .error { display: flex; align-items: center; gap: 12px; padding: 8px 22px; background: var(--bad-soft); color: var(--bad); font-size: 13px; border-bottom: 1px solid var(--line); }
  .error span { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .error button { background: none; color: inherit; padding: 2px 6px; }
  .view { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: auto; }
</style>
