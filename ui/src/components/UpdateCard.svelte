<script lang="ts">
  import { onMount } from "svelte";
  import { t, colon } from "../lib/i18n";
  import { api, subscribeUpdate, type UpdateState } from "../lib/tauri";

  /** 设置页"更新"一节：当前版本、自动检查开关、检查 / 安装按钮与状态行。状态以 Rust 为准（挂载时读一次，之后订阅推送）。 */
  let us = $state<UpdateState>({ state: "idle" });
  let version = $state("—");
  let autoCheck = $state(true);
  let err = $state("");

  const busy = $derived(us.state === "checking" || us.state === "downloading" || us.state === "installing");
  /** 字节 → MB，保留一位小数。 */
  const mb = (n: number) => (n / 1048576).toFixed(1);
  const progress = (s: Extract<UpdateState, { state: "downloading" }>) =>
    s.total != null && s.total > 0 ? `${mb(s.downloaded)} / ${mb(s.total)} MB` : `${mb(s.downloaded)} MB`;

  onMount(() => {
    let un: (() => void) | undefined;
    let gone = false;
    api.snapshot().then((s) => { version = s.version; }).catch(() => {});
    api.getSettings().then((s) => { autoCheck = s.updates.autoCheck; }).catch(() => {});
    // 先订阅再读一次当前状态，避免两者之间漏掉推送；推送到达后不被稍晚返回的旧读数覆盖。
    let pushed = false;
    subscribeUpdate((s) => { pushed = true; us = s; }).then((u) => { if (gone) u(); else un = u; });
    api.updateStatus().then((s) => { if (!pushed) us = s; }).catch((e) => { err = String(e); });
    return () => { gone = true; un?.(); };
  });

  async function check() {
    err = "";
    try { us = await api.checkUpdate(); } catch (e) { err = String(e); }
  }
  async function install() {
    err = "";
    try { await api.installUpdate(); } catch (e) { err = String(e); }
  }
  async function toggleAuto(e: Event) {
    const want = (e.target as HTMLInputElement).checked;
    try { autoCheck = (await api.setSettings({ updates: { autoCheck: want } })).updates.autoCheck; err = ""; }
    // 单向绑定下 autoCheck 没变 Svelte 不会动 DOM，所以显式把复选框拨回去。
    catch (x) { autoCheck = !want; (e.target as HTMLInputElement).checked = autoCheck; err = String(x); }
  }
</script>

<h3>{t("更新")}</h3>
<div class="row"><span class="k">{t("当前版本")}</span><span class="v mono">{version}</span></div>
<div class="row"><span class="k">{t("自动检查更新")}</span><span class="v"><input class="switch" type="checkbox" data-role="auto-check" aria-label={t("自动检查更新")} checked={autoCheck} onchange={toggleAuto}></span></div>
<div class="row">
  <span class="k"></span>
  <span class="v"><button data-role="check-update" disabled={busy || us.state === "unsupported"} onclick={check}>{t("检查更新")}</button></span>
  <span class="h st" data-role="update-status">
    {#if us.state === "unsupported"}{us.reason === "fromSource" ? t("从源码运行，不自动更新") : t("这种安装方式不支持自动更新")}
    {:else if us.state === "checking"}{t("正在检查…")}
    {:else if us.state === "upToDate"}{t("已是最新版本")}
    {:else if us.state === "available"}{t("有新版本")}{colon()}<span class="mono">{us.version}</span>
    {:else if us.state === "downloading"}{t("正在下载")} <span class="mono">{progress(us)}</span>
    {:else if us.state === "installing"}{t("正在安装…")}
    {:else if us.state === "cancelled"}{t("已取消授权，可以稍后再试")}
    {:else if us.state === "error"}<span class="bad">{us.message}</span>{/if}
  </span>
</div>
{#if us.state === "available"}
  <div class="row"><span class="k"></span><span class="v">
    <button data-role="release-notes" onclick={() => { if (us.state === "available") api.openReleaseNotes(us.version).catch((e) => { err = String(e); }); }}>{t("发布说明")}</button>
    <button data-role="install-update" onclick={install}>{t("下载并安装")}</button>
  </span></div>
{/if}
{#if err}<div class="row"><span class="k"></span><span class="h bad">{err}</span></div>{/if}

<style>
  /* 沿用设置页卡片的行样式（样式是 Settings 的作用域，这里重复必要的几条）。 */
  h3 { margin: 0 0 12px; font-size: 12px; font-weight: 600; letter-spacing: .06em; color: var(--muted); }
  .row { display: flex; align-items: center; gap: 14px; padding: 8px 0; border-top: 1px solid var(--line); }
  .k { width: 140px; color: var(--text-2); } .v { display: flex; align-items: center; gap: 10px; }
  .h { color: var(--muted); font-size: 12px; margin-left: auto; max-width: 60%; text-align: right; overflow-wrap: anywhere; }
  .st { font-size: 13px; }
  .mono { font-family: var(--mono); font-size: 13px; font-variant-numeric: tabular-nums; }
  .bad { color: var(--bad); }
  .switch { appearance: none; width: 36px; height: 20px; border-radius: 999px; background: var(--card-on); border: 0; padding: 0; position: relative; cursor: pointer; margin: 0; }
  .switch::after { content: ""; position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; border-radius: 50%; background: var(--text); transition: left 120ms; }
  .switch:checked { background: var(--accent); }
  .switch:checked::after { left: 18px; background: var(--accent-text); }
  .v > button { background: transparent; box-shadow: inset 0 0 0 1px var(--line-strong); padding: 5px 12px; font-size: 13px; white-space: nowrap; }
  .v > button:hover:not(:disabled) { background: var(--card-on); }
</style>
