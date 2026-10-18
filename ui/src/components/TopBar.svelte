<script lang="ts">
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  import Icon from "./Icon.svelte";
  let { page, snapshot, dirty, onNavigate }: { page: "keymap" | "settings"; snapshot: UiSnapshot | null; dirty: boolean; onNavigate: (p: "keymap" | "settings") => void } = $props();
  const status = $derived(!snapshot ? t("未连接") : !snapshot.connected ? t("未连接") : snapshot.paused ? t("空闲中，已交还控制") : snapshot.online === false ? t("接收器在，本体关机") : t("已连接"));
  const ok = $derived(!!snapshot?.connected && snapshot.online !== false && !snapshot.paused);
</script>

<header>
  <div class="logo">
    <!-- 光环标志：一个环加中心点，不是菜单按钮。 -->
    <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true"><circle cx="9" cy="9" r="6.5" fill="none" stroke="var(--accent)" stroke-width="2"/><circle cx="9" cy="9" r="2" fill="var(--accent)"/></svg>
    HALOKEY
  </div>
  <nav>
    <button class:on={page === "keymap"} onclick={() => onNavigate("keymap")}><Icon name="keyboard" /> {t("键位")}{#if dirty}<i class="dot" title={t("未保存")}></i>{/if}</button>
    <button class:on={page === "settings"} onclick={() => onNavigate("settings")}><Icon name="settings" /> {t("设置")}</button>
  </nav>
  <div class="status">
    <div class:ok>● {status}</div>
    {#if snapshot?.battery}<div class="muted"><Icon name="battery" size={15} level={snapshot.battery[0]} /> {snapshot.battery[0] ?? "?"}%</div>{/if}
  </div>
</header>

<style>
  header { display: flex; align-items: center; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  .logo { font-weight: 700; letter-spacing: .12em; display: flex; align-items: center; gap: 8px; user-select: none; }
  nav { margin: 0 auto; display: flex; gap: 24px; }
  nav button { background: none; padding: 6px 2px; display: inline-flex; align-items: center; gap: 6px; color: var(--muted); border-radius: 0; border-bottom: 2px solid transparent; position: relative; }
  nav button:hover { background: none; color: var(--text); }
  nav button.on { color: var(--text); border-bottom-color: var(--accent); }
  .dot { display: inline-block; margin-left: 6px; width: 7px; height: 7px; border-radius: 50%; background: var(--accent); vertical-align: 2px; }
  .status { text-align: right; font-size: 13px; color: var(--muted); line-height: 1.3; }
  .status .ok { color: var(--ok); }
</style>
