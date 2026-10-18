<script lang="ts">
  import { onMount } from "svelte";
  import { t, lang, setLang, type Lang } from "../lib/i18n";
  import { api, type UiSnapshot, type UiDeviceInfo, type UiPower, type HaloSettings } from "../lib/tauri";
  import Icon from "../components/Icon.svelte";
  import { getTheme, setTheme, type Theme } from "../lib/theme";
  let theme = $state<Theme>(getTheme());
  const THEMES: [Theme, string][] = [["system", "跟随系统"], ["dark", "深色"], ["light", "浅色"]];
  const onOff = (v: boolean | number | null | undefined) => (v == null ? "—" : v ? `● ${t("已开启")}` : `○ ${t("已关闭")}`);
  let { snapshot }: { snapshot: UiSnapshot | null } = $props();
  let info = $state<UiDeviceInfo | null>(null);
  let power = $state<UiPower | null>(null);
  let idle = $state(5);
  let autostart = $state(false);
  let note = $state("");
  let halo = $state<HaloSettings>({ position: "pointer", timeoutSeconds: 4, size: "medium" });
  let haloError = $state("");
  const SIZES: [HaloSettings["size"], string][] = [["small", "小"], ["medium", "中"], ["large", "大"]];
  const IDLE = [0, 1, 5, 10, 15, 30, 60];
  const modeName = ["全灭", "全亮", "工作"];
  async function load() {
    try { [info, power] = await Promise.all([api.deviceInfo(), api.powerInfo()]); } catch (e) { note = String(e); }
    try { const s = await api.getSettings(); idle = s.idleMinutes; halo = s.halo; autostart = await api.getAutostart(); } catch { /* mock */ }
  }
  onMount(load);
  const sleepText = (s: number | null) => s == null ? "—" : s === 0 ? t("永不") : `${s / 3600} h`;
  async function changeIdle(e: Event) {
    idle = Number((e.target as HTMLSelectElement).value);
    try { await api.setIdleMinutes(idle); } catch (err) { note = String(err); }
  }
  async function setHalo(patch: Partial<HaloSettings>) {
    try { halo = (await api.setSettings({ halo: patch })).halo; haloError = ""; } catch (e) { haloError = String(e); }
  }
  async function changeAutostart() {
    try { autostart = await api.setAutostart(autostart); } catch (e) { note = String(e); }
  }
</script>

<div class="page">
  <div class="ph"><h2>{t("设置")}</h2><label class="lang"><Icon name="globe" size={15} /> {t("界面语言")} <select value={lang()} onchange={(e) => setLang((e.target as HTMLSelectElement).value as Lang)}><option value="en">English</option><option value="zh">简体中文</option><!-- i18n-ignore: 语言自称 --><option value="ja">日本語</option></select></label></div>

  <section class="card">
    <h3>{t("Vibe Key")} <span class="r"><button onclick={load}>{t("刷新状态")}</button></span></h3>
    <div class="row"><span class="k">{t("状态")}</span><span class="v">{snapshot?.connected ? "● " + (snapshot.online === false ? t("本体离线") : t("本体在线")) : "○ " + t("未连接")}{#if snapshot?.battery} · <Icon name="battery" size={15} level={snapshot.battery[0]} /> {snapshot.battery[0] ?? "?"}% · {snapshot.battery[1]} mV{/if}</span><span class="h">{t("心跳")} {snapshot?.heartbeats ?? 0} · {t("回显")} {snapshot?.echoes ?? 0}</span></div>
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
      <div class="row"><span class="k">{t("收音")}</span><span class="v">{onOff(power?.micEnabled)}</span></div>
      <div class="row"><span class="k">{t("硬件降噪")}</span><span class="v">{onOff(power?.noiseReduction)}</span></div>
    </section>
  </div>

  <div class="two">
    <section class="card">
      <h3>{t("电源")}</h3>
      <div class="row"><span class="k">{t("休眠 / 自动关机")}</span><span class="v mono">{sleepText(power?.sleepDelay ?? null)}</span></div>
      <div class="row"><span class="k">{t("待机延时")}</span><span class="v mono">{power?.standbyDelay ?? "—"} s</span></div>
    </section>
    <section class="card">
      <h3>{t("光环转盘")}</h3>
      <div class="row"><span class="k">{t("出现位置")}</span><span class="v"><span class="seg">
        <button class:on={halo.position === "pointer"} onclick={() => setHalo({ position: "pointer" })}>{t("指针周围")}</button>
        <button class:on={halo.position === "center"} onclick={() => setHalo({ position: "center" })}>{t("屏幕中央")}</button></span></span></div>
      <div class="row"><span class="k">{t("自动关闭")}</span><span class="v"><select data-role="halo-timeout" value={String(halo.timeoutSeconds)} onchange={(e) => setHalo({ timeoutSeconds: Number((e.target as HTMLSelectElement).value) })}>{#each [2, 4, 6, 10] as s}<option value={String(s)}>{s} {t("秒")}</option>{/each}</select></span></div>
      <div class="row"><span class="k">{t("大小")}</span><span class="v"><span class="seg">
        {#each SIZES as [v, name]}<button class:on={halo.size === v} onclick={() => setHalo({ size: v })}>{t(name)}</button>{/each}</span></span></div>
      {#if haloError}<p class="bad">{haloError}</p>{/if}
    </section>
  </div>
  <section class="card">
    <h3>{t("通用")}</h3>
    <div class="row"><span class="k">{t("登录时启动")}</span><span class="v"><input class="switch" type="checkbox" data-role="autostart" aria-label={t("登录时启动")} bind:checked={autostart} onchange={changeAutostart}></span></div>
    <div class="row"><span class="k">{t("外观")}</span><span class="v"><span class="seg">
      {#each THEMES as [v, name]}<button data-theme-choice={v} class:on={theme === v} onclick={() => { theme = v; setTheme(v); }}>{t(name)}</button>{/each}</span></span></div>
  </section>
  {#if note}<p class="bad">{note}</p>{/if}
  <p class="about mono">halokey 0.0.1 · {snapshot?.configPath ?? "—"}</p>
</div>

<style>
  .page { padding: 18px 22px; display: flex; flex-direction: column; gap: 14px; }
  .ph { display: flex; align-items: center; } .ph h2 { margin: 0; font-size: 18px; } .lang { margin-left: auto; color: var(--muted); }
  .card { background: var(--card); border-radius: var(--r-lg); padding: 14px 18px; }
  /* 卡片标题做成小标题（eyebrow）：12px、muted、字距，和正文拉开层级。 */
  .card h3 .r button { background: transparent; box-shadow: inset 0 0 0 1px var(--line-strong); padding: 4px 12px; font-size: 13px; font-weight: 500; letter-spacing: 0; color: var(--text-2); }
  .card h3 .r button:hover { background: var(--card-on); }
  .card h3 { margin: 0 0 12px; font-size: 12px; font-weight: 600; letter-spacing: .06em; color: var(--muted); display: flex; align-items: center; } .card h3 .r { margin-left: auto; }
  .about { color: var(--muted); font-size: 12px; margin: 4px 0 0; }
  /* 开关：原生 checkbox 画成滑动开关。 */
  .switch { appearance: none; width: 36px; height: 20px; border-radius: 999px; background: var(--card-on); border: 0; padding: 0; position: relative; cursor: pointer; margin: 0; }
  .switch::after { content: ""; position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%; background: var(--text); transition: left 120ms; }
  .switch:checked { background: var(--accent); }
  .switch:checked::after { left: 18px; background: var(--accent-text); }
  .row { display: flex; align-items: center; gap: 14px; padding: 8px 0; border-top: 1px solid var(--line); }
  .row:first-of-type { border-top: 0; }
  .k { width: 140px; color: var(--text-2); } .v { display: flex; align-items: center; gap: 10px; flex: 1; } .h { color: var(--muted); font-size: 12px; margin-left: auto; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .mono { font-family: var(--mono); font-size: 13px; font-variant-numeric: tabular-nums; }
  .muted { color: var(--muted); font-size: 13px; } .bad { color: var(--bad); }
  .seg { display: inline-flex; background: var(--card); border-radius: var(--r-md); padding: 3px; gap: 2px; }
  .seg button { background: none; padding: 5px 12px; font-size: 13px; }
  .seg button:hover { background: var(--card-on); }
  .seg button.on { background: var(--card-on); color: var(--accent-ink); }
</style>
