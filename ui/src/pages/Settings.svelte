<script lang="ts">
  import { onMount } from "svelte";
  import { t, lang, setLang } from "../lib/i18n";
  import { api, type UiSnapshot, type UiDeviceInfo, type UiPower } from "../lib/tauri";
  let { snapshot }: { snapshot: UiSnapshot | null } = $props();
  let info = $state<UiDeviceInfo | null>(null);
  let power = $state<UiPower | null>(null);
  let idle = $state(5);
  let autostart = $state(false);
  let note = $state("");
  const IDLE = [0, 1, 5, 10, 15, 30, 60];
  const modeName = ["全灭", "全亮", "工作"];
  async function load() {
    try { [info, power] = await Promise.all([api.deviceInfo(), api.powerInfo()]); } catch (e) { note = String(e); }
    try { idle = (await api.getSettings()).idleMinutes; autostart = await api.getAutostart(); } catch { /* mock */ }
  }
  onMount(load);
  const sleepText = (s: number | null) => s == null ? "—" : s === 0 ? t("永不") : `${s / 3600} h`;
  async function changeIdle(e: Event) {
    idle = Number((e.target as HTMLSelectElement).value);
    try { await api.setIdleMinutes(idle); } catch (err) { note = String(err); }
  }
  async function changeAutostart() {
    try { autostart = await api.setAutostart(autostart); } catch (e) { note = String(e); }
  }
</script>

<div class="page">
  <div class="ph"><h2>{t("设置")}</h2><label class="lang">🌐 {t("界面语言")} <select value={lang()} onchange={(e) => setLang((e.target as HTMLSelectElement).value as "zh" | "en")}><option value="zh">简体中文</option><option value="en">English</option></select></label></div>
  <p class="muted">{t("第 1 期：设置页只读；写入项在后续计划中")}</p>

  <section class="card">
    <h3>{t("Vibe Key")} <span class="r"><button onclick={load}>{t("刷新状态")}</button></span></h3>
    <div class="row"><span class="k">{t("状态")}</span><span class="v">{snapshot?.connected ? "● " + (snapshot.online === false ? t("本体离线") : t("本体在线")) : "○ " + t("未连接")}{#if snapshot?.battery} · 🔋 {snapshot.battery[0] ?? "?"}% · {snapshot.battery[1]} mV{/if}</span><span class="h">{t("心跳")} {snapshot?.heartbeats ?? 0} · {t("回显")} {snapshot?.echoes ?? 0}</span></div>
    <div class="row"><span class="k">{t("固件")}</span><span class="v mono">{info?.firmware ?? "—"} / {info?.dongleFirmware ?? "—"}</span></div>
    <div class="row"><span class="k">{t("空闲后交还控制")}</span><span class="v"><select data-role="idle" value={String(idle)} onchange={changeIdle}>{#each IDLE as m}<option value={String(m)}>{m === 0 ? t("永不") : `${m} ${t("分钟")}`}</option>{/each}</select></span></div>
  </section>

  <div class="two">
    <section class="card">
      <h3>{t("灯光")}</h3>
      <div class="row"><span class="k">{t("灯光模式")}</span><span class="v">{info?.lighting ? t(modeName[info.lighting.mode] ?? "—") : "—"}</span></div>
      <div class="row"><span class="k">{t("全亮亮度")}</span><span class="v mono">{info?.lighting?.brightness ?? "—"} / 20</span></div>
      <div class="row"><span class="k"></span><span class="v muted">{t("各灯效果在键位页按 Layer 设置")}</span></div>
    </section>
    <section class="card">
      <h3>{t("麦克风")}</h3>
      <div class="row"><span class="k">{t("收音")}</span><span class="v">{power?.micEnabled == null ? "—" : power.micEnabled ? "✓" : "✕"}</span></div>
      <div class="row"><span class="k">{t("硬件降噪")}</span><span class="v">{power?.noiseReduction == null ? "—" : power.noiseReduction ? "✓" : "✕"}</span></div>
    </section>
  </div>

  <div class="two">
    <section class="card">
      <h3>{t("电源")}</h3>
      <div class="row"><span class="k">{t("休眠 / 自动关机")}</span><span class="v">{sleepText(power?.sleepDelay ?? null)}</span></div>
      <div class="row"><span class="k">{t("待机延时")}</span><span class="v mono">{power?.standbyDelay ?? "—"} s</span></div>
    </section>
    <section class="card">
      <h3>{t("通用")}</h3>
      <div class="row"><span class="k">{t("登录时启动")}</span><span class="v"><input type="checkbox" bind:checked={autostart} onchange={changeAutostart}></span></div>
      <div class="row"><span class="k">{t("当前文件")}</span><span class="v mono">{snapshot?.configPath ?? "—"}</span></div>
      <div class="row"><span class="k">{t("关于")}</span><span class="v mono">halokey 0.0.1</span></div>
    </section>
  </div>
  {#if note}<p class="bad">{note}</p>{/if}
</div>

<style>
  .page { padding: 18px 22px; display: flex; flex-direction: column; gap: 14px; }
  .ph { display: flex; align-items: center; } .ph h2 { margin: 0; font-size: 18px; } .lang { margin-left: auto; color: var(--muted); }
  .card { background: var(--panel); border-radius: 12px; padding: 14px 18px; }
  .card h3 { margin: 0 0 10px; font-size: 14px; display: flex; align-items: center; } .card h3 .r { margin-left: auto; }
  .row { display: flex; align-items: center; gap: 14px; padding: 8px 0; border-top: 1px solid var(--line); }
  .row:first-of-type { border-top: 0; }
  .k { width: 140px; color: #c9c9ce; } .v { display: flex; align-items: center; gap: 10px; flex: 1; } .h { color: var(--muted); font-size: 12.5px; margin-left: auto; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .mono { font-family: var(--mono); font-size: 12.5px; }
  .muted { color: var(--muted); font-size: 13px; } .bad { color: var(--bad); }
</style>
