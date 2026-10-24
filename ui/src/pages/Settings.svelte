<script lang="ts">
  import { onMount } from "svelte";
  import { t, lang, setLang, colon, type Lang } from "../lib/i18n";
  import { api, type UiSnapshot, type UiDeviceInfo, type UiPower, type HaloSettings, type UiLighting, type DeviceKeys } from "../lib/tauri";
  import Icon from "../components/Icon.svelte";
  import UpdateCard from "../components/UpdateCard.svelte";
  import { getTheme, setTheme, type Theme } from "../lib/theme";
  import { entriesLabel, CONTROL_NAMES } from "../model/labels";
  import type { HostKeymap } from "../model/keymap";

  /** 设置页：每项改动立即写设备 / 设置文件并读回；失败在该行右侧红字（§6 / §9），没有保存按钮。 */
  let { snapshot, dirty = false, hasBacklight = false, onKeymapReplaced }: { snapshot: UiSnapshot | null; dirty?: boolean; hasBacklight?: boolean; onKeymapReplaced?: (map: HostKeymap) => void } = $props();
  let info = $state<UiDeviceInfo | null>(null);
  let power = $state<UiPower | null>(null);
  let idle = $state(5);
  let autostart = $state(false);
  let closeToTray = $state(true);
  let note = $state("");
  let halo = $state<HaloSettings>({ position: "pointer", timeoutSeconds: 4, size: "medium" });
  let theme = $state<Theme>(getTheme());
  /** 每行一条错误（键 = 行名）。 */
  let rowError = $state<Record<string, string>>({});
  let audioSource = $state("");
  let deviceKeys = $state<DeviceKeys | null>(null);
  let writeReport = $state<string[]>([]);
  const THEMES: [Theme, string][] = [["system", "跟随系统"], ["dark", "深色"], ["light", "浅色"]];
  const SIZES: [HaloSettings["size"], string][] = [["small", "小"], ["medium", "中"], ["large", "大"]];
  const IDLE = [0, 1, 5, 10, 15, 30, 60];
  const SLEEP = [0, 3600, 7200, 10800, 14400];
  const STANDBY = [60, 120, 300, 600, 1800];
  const modeName = ["全灭", "全亮", "工作"];
  const sleepLabel = (s: number) => (s === 0 ? t("永不") : `${s / 3600} ${t("小时")}`);

  async function load() {
    try { [info, power] = await Promise.all([api.deviceInfo(), api.powerInfo()]); } catch (e) { note = String(e); }
    try { const s = await api.getSettings(); idle = s.idleMinutes; halo = s.halo; closeToTray = s.closeToTray; autostart = await api.getAutostart(); } catch { /* mock */ }
  }
  onMount(load);

  /** 正在写入的行：写入期间禁用该行控件，避免 15 s 超时窗口里排队重复写。 */
  let busy = $state<Record<string, boolean>>({});
  /** 跑一次设备写入：成功清掉该行错误并把读回值交给 apply；失败把错误放到该行并 revert（控件回到写入前的值）。 */
  async function write<T>(row: string, run: () => Promise<T>, apply: (v: T) => void, revert?: () => void) {
    busy = { ...busy, [row]: true };
    try { const v = await run(); rowError = { ...rowError, [row]: "" }; apply(v); }
    catch (e) { rowError = { ...rowError, [row]: String(e) }; revert?.(); }
    finally { busy = { ...busy, [row]: false }; }
  }
  const online = $derived(!!snapshot?.connected);
  // 全局灯光：四盏灯的记录原样回传，只改 mode / brightness（Rust 要求四盏齐全）。
  const setLightingGlobal = (patch: Partial<Pick<UiLighting, "mode" | "brightness">>) => {
    if (!info?.lighting) return;
    const desired = { ...info.lighting, ...patch };
    return write("lighting", () => api.setLighting(desired), (l) => { if (info) info = { ...info, lighting: l }; });
  };
  // 下拉：失败时把 DOM 值拨回当前读到的值（Svelte 不会重设没变化的 value）。
  const setSleep = (el: HTMLSelectElement) => write("sleep", () => api.setSleepDelay(Number(el.value)), (v) => { if (power) power = { ...power, sleepDelay: v }; }, () => { el.value = String(power?.sleepDelay ?? ""); });
  const setStandby = (el: HTMLSelectElement) => write("standby", () => api.setStandbyDelay(Number(el.value)), (v) => { if (power) power = { ...power, standbyDelay: v }; }, () => { el.value = String(power?.standbyDelay ?? ""); });
  // 没有"收音"一行：`2a` 标志既改不了也不反映麦克风能否录音（2026-10-20 实测），见 docs/02 §8。
  // 开关的 DOM 状态被点击翻转过了，读回后要显式写回，否则读回值没变时界面会停在"开"。
  const setNoise = (el: HTMLInputElement) => write("noise", () => api.setNoiseReduction(el.checked ? 1 : 0), (v) => { el.checked = !!v; if (power) power = { ...power, noiseReduction: v }; }, () => { el.checked = !!power?.noiseReduction; });
  const setDefaultInput = () => write("audio", () => api.setDefaultAudioInput(), (name) => { audioSource = name; });
  const readDevice = () => write("deviceKeys", () => api.readDeviceKeys(), (k) => { deviceKeys = k; writeReport = []; });
  const writeDevice = () => write("deviceKeys", () => api.writeDeviceKeys(), (r) => { writeReport = r; deviceKeys = null; });
  async function changeIdle(e: Event) {
    idle = Number((e.target as HTMLSelectElement).value);
    try { await api.setSettings({ idleMinutes: idle }); rowError = { ...rowError, idle: "" }; } catch (err) { rowError = { ...rowError, idle: String(err) }; }
  }
  async function setHalo(patch: Partial<HaloSettings>) {
    try { halo = (await api.setSettings({ halo: patch })).halo; rowError = { ...rowError, halo: "" }; } catch (e) { rowError = { ...rowError, halo: String(e) }; }
  }
  async function setClose(tray: boolean) {
    try { closeToTray = (await api.setSettings({ closeToTray: tray })).closeToTray; rowError = { ...rowError, close: "" }; } catch (e) { rowError = { ...rowError, close: String(e) }; }
  }
  // 键位配置卡：三种"替换键位文件"的操作共用一个流程；取消（null）不提示。
  let fileNote = $state("");
  let fileError = $state("");
  async function replaceWith(run: () => Promise<HostKeymap | null>) {
    fileNote = ""; fileError = "";
    try {
      const map = await run();
      if (!map) return;
      onKeymapReplaced?.(map);
      fileNote = t("已替换键位文件");
    } catch (e) { fileError = String(e); }
  }
  async function exportFile() {
    fileNote = ""; fileError = "";
    try { const p = await api.exportKeymap(); if (p) fileNote = `${t("已导出到")} ${p}`; } catch (e) { fileError = String(e); }
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
    <div class="row"><span class="k">{t("空闲后交还控制")}</span><span class="v"><select data-role="idle" value={String(idle)} onchange={changeIdle}>{#each IDLE as m}<option value={String(m)}>{m === 0 ? t("永不") : `${m} ${t("分钟")}`}</option>{/each}</select></span>{#if rowError.idle}<span class="h bad">{rowError.idle}</span>{/if}</div>
    <div class="row"><span class="k">{t("设备自身键位")}</span><span class="v"><button data-role="write-device" disabled={!snapshot?.connected} onclick={writeDevice}>{t("写入设备")}</button><button data-role="read-device" disabled={!snapshot?.connected} onclick={readDevice}>{t("从设备读取")}</button></span>{#if rowError.deviceKeys}<span class="h bad">{rowError.deviceKeys}</span>{/if}</div>
    {#if deviceKeys}
      <ul class="keys">{#each deviceKeys as [index, entries]}<li><span class="k">{t(CONTROL_NAMES[index] ?? "")}</span><span class="mono">{entriesLabel(entries.map(([type, code]) => ({ type, code })))}</span></li>{/each}</ul>
    {/if}
    {#if writeReport.length}<ul class="keys">{#each writeReport as line}<li class="mono">{line}</li>{/each}</ul>{/if}
    <details class="help"><summary>{t("连接帮助")}</summary>
      <ol>
        <li>{t("Linux 需要 udev 规则")}{colon()}<span class="mono">/etc/udev/rules.d/70-halokey.rules</span>{t("（.deb 会自动安装；手装见 packaging/linux）")}</li>
        <li>{t("装好规则后重新插拔接收器")}</li>
        <li>{t("本体待机后按任意键唤醒；电源键长按会关机")}</li>
      </ol>
    </details>
  </section>

  <div class="two">
    <section class="card">
      <h3>{t("灯光")}</h3>
      <div class="row"><span class="k">{t("灯光模式")}</span><span class="v"><span class="seg">{#each [0, 1, 2] as m}<button data-light-mode={m} class:on={info?.lighting?.mode === m} disabled={!online || !info?.lighting || busy.lighting} onclick={() => setLightingGlobal({ mode: m })}>{t(modeName[m]!)}</button>{/each}</span></span>{#if rowError.lighting}<span class="h bad">{rowError.lighting}</span>{/if}</div>
      <div class="row"><span class="k">{t("全亮亮度")}</span><span class="v"><input type="range" data-role="all-on" min="0" max="20" value={info?.lighting?.brightness ?? 0} disabled={!online || !info?.lighting || busy.lighting} onchange={(e) => setLightingGlobal({ brightness: Number((e.target as HTMLInputElement).value) })} /><span class="mono">{info?.lighting?.brightness ?? "—"} / 20</span></span></div>
      <div class="row"><span class="k"></span><span class="v muted">{hasBacklight ? t("工作：按键位页的 Layer 背光；全灭 / 全亮时忽略分层背光") : t("各灯效果在键位页按 Layer 设置")}</span></div>
    </section>
    <section class="card">
      <h3>{t("麦克风")}</h3>
      <div class="row"><span class="k">{t("硬件降噪")}</span><span class="v"><input class="switch" type="checkbox" data-role="noise" aria-label={t("硬件降噪")} checked={!!power?.noiseReduction} disabled={!online || power?.noiseReduction == null || busy.noise} onchange={(e) => setNoise(e.target as HTMLInputElement)}></span>{#if rowError.noise}<span class="h bad">{rowError.noise}</span>{/if}</div>
      <div class="row"><span class="k">{t("系统默认输入")}</span><span class="v"><button data-role="default-input" disabled={busy.audio} onclick={setDefaultInput}>{t("设为系统默认输入")}</button></span>{#if rowError.audio}<span class="h bad">{rowError.audio}</span>{:else if audioSource}<span class="h ok" data-role="audio-source" title={audioSource}>✓ {t("已设为系统默认输入")}</span>{/if}</div>
    </section>
  </div>

  <div class="two">
    <section class="card">
      <h3>{t("电源")}</h3>
      <div class="row"><span class="k">{t("休眠 / 自动关机")}</span><span class="v"><select data-role="sleep" value={String(power?.sleepDelay ?? "")} disabled={!online || power?.sleepDelay == null || busy.sleep} onchange={(e) => setSleep(e.target as HTMLSelectElement)}>{#if power?.sleepDelay == null}<option value="">—</option>{/if}{#each SLEEP as s}<option value={String(s)}>{sleepLabel(s)}</option>{/each}</select></span>{#if rowError.sleep}<span class="h bad">{rowError.sleep}</span>{/if}</div>
      <div class="row"><span class="k">{t("待机延时")}</span><span class="v"><select data-role="standby" value={String(power?.standbyDelay ?? "")} disabled={!online || power?.standbyDelay == null || busy.standby} onchange={(e) => setStandby(e.target as HTMLSelectElement)}>{#if power?.standbyDelay == null}<option value="">—</option>{/if}{#each STANDBY as s}<option value={String(s)}>{s} {t("秒")}</option>{/each}</select></span>{#if rowError.standby}<span class="h bad">{rowError.standby}</span>{/if}</div>
    </section>
    <section class="card">
      <h3>{t("光环转盘")}</h3>
      <div class="row"><span class="k">{t("出现位置")}</span><span class="v"><span class="seg">
        <button class:on={halo.position === "pointer"} onclick={() => setHalo({ position: "pointer" })}>{t("指针周围")}</button>
        <button class:on={halo.position === "center"} onclick={() => setHalo({ position: "center" })}>{t("屏幕中央")}</button></span></span></div>
      <div class="row"><span class="k">{t("自动关闭")}</span><span class="v"><select data-role="halo-timeout" value={String(halo.timeoutSeconds)} onchange={(e) => setHalo({ timeoutSeconds: Number((e.target as HTMLSelectElement).value) })}>{#each [2, 4, 6, 10] as s}<option value={String(s)}>{s} {t("秒")}</option>{/each}</select></span></div>
      <div class="row"><span class="k">{t("大小")}</span><span class="v"><span class="seg">
        {#each SIZES as [v, name]}<button class:on={halo.size === v} onclick={() => setHalo({ size: v })}>{t(name)}</button>{/each}</span></span>{#if rowError.halo}<span class="h bad">{rowError.halo}</span>{/if}</div>
    </section>
  </div>

  <section class="card">
    <h3>{t("键位配置")}</h3>
    <div class="row"><span class="k">{t("当前文件")}</span><span class="v mono">{snapshot?.configPath ?? "—"}</span></div>
    <div class="row"><span class="k"></span><span class="v wrap">
      <button data-role="import" disabled={dirty} onclick={() => replaceWith(api.importKeymap)}>{t("导入…")}</button>
      <button data-role="export" onclick={exportFile}>{t("导出…")}</button>
      <button data-role="load-default" disabled={dirty} onclick={() => replaceWith(api.loadDefaultKeymap)}>{t("载入默认键位")}</button>
      <button data-role="from-device" disabled={dirty || !snapshot?.connected} onclick={() => replaceWith(api.keymapFromDevice)}>{t("从设备键位生成")}</button>
    </span>{#if fileError}<span class="h bad">{fileError}</span>{:else if fileNote}<span class="h">{fileNote}</span>{:else if dirty}<span class="h">{t("先保存或撤销键位页的改动")}</span>{/if}</div>
    {#if dirty && (fileError || fileNote)}<div class="row"><span class="k"></span><span class="v muted">{t("先保存或撤销键位页的改动")}</span></div>{/if}
  </section>

  <section class="card">
    <h3>{t("通用")}</h3>
    <div class="row"><span class="k">{t("登录时启动")}</span><span class="v"><input class="switch" type="checkbox" data-role="autostart" aria-label={t("登录时启动")} bind:checked={autostart} onchange={changeAutostart}></span></div>
    <div class="row"><span class="k">{t("关闭窗口时")}</span><span class="v"><span class="seg">
      <button data-close="tray" class:on={closeToTray} onclick={() => setClose(true)}>{t("留在托盘")}</button>
      <button data-close="quit" class:on={!closeToTray} onclick={() => setClose(false)}>{t("退出")}</button></span></span>{#if rowError.close}<span class="h bad">{rowError.close}</span>{/if}</div>
    <div class="row"><span class="k">{t("外观")}</span><span class="v"><span class="seg">
      {#each THEMES as [v, name]}<button data-theme-choice={v} class:on={theme === v} onclick={() => { theme = v; setTheme(v); }}>{t(name)}</button>{/each}</span></span></div>
  </section>
  <section class="card" id="updates"><UpdateCard /></section>
  {#if note}<p class="bad">{note}</p>{/if}
  <p class="about mono">halokey {snapshot?.version ?? "—"} · {snapshot?.configPath ?? "—"}</p>
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
  .k { width: 140px; color: var(--text-2); } .v { display: flex; align-items: center; gap: 10px; flex: 1; }
  /* 右侧提示最多占一半宽并可换行，不许把左边的控件挤扁。 */
  .h { color: var(--muted); font-size: 12px; margin-left: auto; max-width: 45%; text-align: right; overflow-wrap: anywhere; }
  .h.ok { color: var(--ok); }
  .v > button, .v > select, .v > .seg { flex: none; white-space: nowrap; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .mono { font-family: var(--mono); font-size: 13px; font-variant-numeric: tabular-nums; }
  .muted { color: var(--muted); font-size: 13px; } .bad { color: var(--bad); }
  .seg { display: inline-flex; background: var(--bg); border-radius: var(--r-md); padding: 3px; gap: 2px; }
  .seg button { background: none; padding: 5px 12px; font-size: 13px; }
  .seg button:hover { background: var(--card-on); }
  .seg button.on { background: var(--card-on); color: var(--accent-ink); }
  .keys { list-style: none; margin: 4px 0 6px 154px; padding: 0; display: flex; flex-direction: column; gap: 2px; font-size: 13px; }
  .keys li { display: flex; gap: 14px; } .keys .k { width: 100px; }
  .help { margin-top: 8px; font-size: 13px; color: var(--muted); } .help summary { cursor: pointer; } .help ol { margin: 6px 0 0; padding-left: 20px; }
  .h.bad { color: var(--bad); } .v input[type="range"] { width: 160px; } .v.wrap { flex-wrap: wrap; }
  /* 卡片里的动作按钮：卡片底上 card 色不可见，用描边。 */
  .v > button { background: transparent; box-shadow: inset 0 0 0 1px var(--line-strong); padding: 5px 12px; font-size: 13px; }
  .v > button:hover { background: var(--card-on); }
</style>
