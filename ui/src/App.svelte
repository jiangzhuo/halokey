<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "./components/TopBar.svelte";
  import Keymap from "./pages/Keymap.svelte";
  import Settings from "./pages/Settings.svelte";
  import { api, subscribe, type UiSnapshot } from "./lib/tauri";
  import { initialDraft, loadSaved, type Draft } from "./model/draft";

  let page = $state<"keymap" | "settings">("keymap");
  let snapshot = $state<UiSnapshot | null>(null);
  let draft = $state<Draft>(initialDraft());
  let langTick = $state(0);

  async function refresh() {
    try { snapshot = await api.snapshot(); } catch (e) { console.error(e); }
    if (!draft.map && snapshot?.connected) try { draft = loadSaved(draft, await api.getKeymap()); } catch (e) { console.error(e); }
  }
  onMount(() => {
    refresh();
    const timer = setInterval(refresh, 2000);
    const relang = () => { langTick++; };
    window.addEventListener("halokey-lang", relang);
    let un: (() => void) | undefined;
    subscribe((e) => { if (e.kind === "connected") api.getKeymap().then((m) => { draft = loadSaved(draft, m); }); }).then((u) => { un = u; });
    return () => { clearInterval(timer); window.removeEventListener("halokey-lang", relang); un?.(); };
  });
</script>

{#key langTick}
  <TopBar {page} {snapshot} dirty={draft.dirty} onNavigate={(p) => (page = p)} />
  {#if page === "keymap"}
    <Keymap bind:draft {snapshot} />
  {:else}
    <Settings {snapshot} />
  {/if}
{/key}
