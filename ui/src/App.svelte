<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "./components/TopBar.svelte";
  import Keymap from "./pages/Keymap.svelte";
  import Settings from "./pages/Settings.svelte";
  import HaloEditor from "./pages/HaloEditor.svelte";
  import { api, subscribe, type UiSnapshot } from "./lib/tauri";
  import { initialDraft, loadSaved, assignAction, select, type Draft, type Selection } from "./model/draft";

  let page = $state<"keymap" | "settings" | "halo">("keymap");
  // 光环编辑器：记住从哪个手势进来的，新建成功后把新光环分配回去。
  let editing = $state<{ id: string | null; wasNew: boolean; returnTo: Selection } | null>(null);
  function openHalo(id: string | null) {
    editing = { id, wasNew: id === null, returnTo: draft.selection };
    page = "halo";
  }
  function closeHalo(savedId: string | null) {
    if (editing?.wasNew && savedId) draft = assignAction(select(draft, editing.returnTo), { halo: { _0: savedId } });
    editing = null;
    page = "keymap";
  }
  let snapshot = $state<UiSnapshot | null>(null);
  let draft = $state<Draft>(initialDraft());
  let langTick = $state(0);
  let activeLayers = $state<number[]>([]);

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
    }).then((u) => { un = u; });
    return () => { clearInterval(timer); window.removeEventListener("halokey-lang", relang); un?.(); };
  });
</script>

{#key langTick}
<div class="app">
  <TopBar page={page === "halo" ? "keymap" : page} {snapshot} dirty={draft.dirty} onNavigate={(p) => { editing = null; page = p; }} />
  <div class="view">
  {#if page === "keymap"}
    <Keymap bind:draft {snapshot} {activeLayers} onEditHalo={openHalo} />
  {:else if page === "halo"}
    <HaloEditor bind:draft haloId={editing?.id ?? null} onClose={closeHalo} />
  {:else}
    <Settings {snapshot} />
  {/if}
  </div>
</div>
{/key}

<style>
  /* 顶栏固定；页面内容在 .view 里滚动（键位页自己再把舞台固定、只滚键码面板）。 */
  .app { display: flex; flex-direction: column; height: 100vh; }
  .view { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: auto; }
</style>
