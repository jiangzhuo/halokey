<script lang="ts">
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  let { page, snapshot, dirty, onNavigate }: { page: "keymap" | "settings"; snapshot: UiSnapshot | null; dirty: boolean; onNavigate: (p: "keymap" | "settings") => void } = $props();
  const status = $derived(!snapshot ? t("未连接") : !snapshot.connected ? t("未连接") : snapshot.paused ? t("空闲中，已交还控制") : snapshot.online === false ? t("接收器在，本体关机") : t("已连接"));
  const ok = $derived(!!snapshot?.connected && snapshot.online !== false && !snapshot.paused);
</script>

<header>
  <div class="logo">☰ HALOKEY</div>
  <nav>
    <button class:on={page === "keymap"} onclick={() => onNavigate("keymap")}>⌨ {t("键位")}{#if dirty}<i class="dot" title={t("未保存")}></i>{/if}</button>
    <button class:on={page === "settings"} onclick={() => onNavigate("settings")}>⚙ {t("设置")}</button>
  </nav>
  <div class="status">
    <div class:ok>● {status}</div>
    {#if snapshot?.battery}<div class="muted">🔋 {snapshot.battery[0] ?? "?"}% · {snapshot.battery[1]} mV</div>{/if}
  </div>
</header>

<style>
  header { display: flex; align-items: center; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  .logo { font-weight: 700; letter-spacing: .12em; }
  nav { margin: 0 auto; display: flex; gap: 24px; }
  nav button { background: none; padding: 6px 2px; color: var(--muted); border-radius: 0; border-bottom: 2px solid transparent; position: relative; }
  nav button.on { color: var(--text); border-bottom-color: var(--accent); }
  .dot { position: absolute; top: 4px; right: -8px; width: 7px; height: 7px; border-radius: 50%; background: var(--accent); }
  .status { text-align: right; font-size: 13px; color: var(--muted); line-height: 1.3; }
  .status .ok { color: var(--ok); }
</style>
