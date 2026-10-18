<script lang="ts">
  import { t } from "../lib/i18n";
  import { LIGHT_NAMES } from "../model/labels";
  import type { LayerLighting, LightEffect, LightMode } from "../model/keymap";
  let { layer, lighting, own, onEffect, onFollow, onClearBase }: { layer: number; lighting: LayerLighting | null; own: boolean; onEffect: (light: number, patch: Partial<LightEffect>) => void; onFollow: (follow: boolean) => void; onClearBase: () => void } = $props();
  let expanded = $state(false);
  const MODES: LightMode[] = ["off", "solid", "breathe"];
  const modeName: Record<LightMode, string> = { off: "关", solid: "常亮", breathe: "呼吸" };
  const summary = $derived(!lighting ? (layer === 0 ? t("未设置（不管灯）") : t("跟随 Layer 0")) : "");
  // 状态条里每盏灯一个紧凑小块：短名 + 模式圆点 + 亮度（关着的不显示亮度）。
  const SHORT = ["1", "2", "3", "⟳"];
</script>

<div class="wrap">
  <div class="strip">
    <span class="title">{t("本层背光")}</span>
    {#if lighting}
      {#each lighting.lights as l, i}
        <span class="lamp" title={`${t(LIGHT_NAMES[i]!)} · ${t(modeName[l.mode])} · ${l.brightness}`}><b>{SHORT[i]}</b><span class="dot" data-mode={l.mode}></span>{#if l.mode !== "off"}<span class="mono">{l.brightness}</span>{/if}</span>
      {/each}
    {:else}
      <span class="sum">{summary}</span>
    {/if}
    {#if layer !== 0 && !own}<span class="muted">▽</span>{/if}
    <button onclick={() => (expanded = !expanded)}>{expanded ? t("收起") : t("编辑…")}</button>
  </div>
  {#if expanded}
    <div class="editor">
      {#each [0, 1, 2, 3] as i}
        {@const l = lighting?.lights[i] ?? { mode: "off", brightness: 0 }}
        <div class="light">
          <span class="name">{t(LIGHT_NAMES[i]!)}</span>
          <span class="seg">
            {#each MODES as m}<button data-mode={m} class:on={l.mode === m} onclick={() => onEffect(i, m === "off" ? { mode: m } : { mode: m, brightness: l.brightness || 20 })}>{t(modeName[m])}</button>{/each}
          </span>
          <input type="range" min="0" max="20" value={l.brightness} disabled={l.mode === "off"} oninput={(e) => onEffect(i, { brightness: Number((e.target as HTMLInputElement).value) })} />
          <span class="mono">{l.brightness}</span>
        </div>
      {/each}
      <div class="foot">
        {#if layer !== 0}
          <label><input type="checkbox" data-role="follow" checked={!own} onchange={(e) => onFollow((e.target as HTMLInputElement).checked)} /> {t("跟随 Layer 0")}</label>
        {:else if lighting}
          <button onclick={onClearBase}>{t("不管灯（删除本层背光）")}</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .wrap { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; }
  .strip { display: flex; align-items: center; gap: 8px; background: var(--card); border-radius: 8px; padding: 6px 10px; font-size: 13px; }
  .title { color: var(--muted); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: #55555d; display: inline-block; }
  .dot[data-mode="solid"] { background: var(--accent); }
  .dot[data-mode="breathe"] { background: var(--accent); animation: breathe 1.8s ease-in-out infinite; }
  .sum { max-width: 360px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lamp { display: inline-flex; align-items: center; gap: 4px; padding: 2px 6px; border-radius: 6px; background: var(--bg); }
  .lamp b { font-weight: 600; font-size: 12px; color: var(--muted); }
  .lamp .mono { font-family: var(--mono); font-size: 12px; }
  .muted { color: var(--muted); }
  .strip button { padding: 4px 10px; font-size: 13px; }
  .editor { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; padding: 10px 14px; display: flex; flex-direction: column; gap: 8px; min-width: 460px; }
  .light { display: grid; grid-template-columns: 48px auto 1fr 28px; align-items: center; gap: 10px; }
  .name { color: var(--muted); font-size: 13px; }
  .seg { display: inline-flex; background: var(--card); border-radius: 8px; padding: 3px; gap: 2px; }
  .seg button { background: none; padding: 4px 10px; font-size: 13px; }
  .seg button.on { background: var(--accent); color: var(--accent-text); }
  .mono { font-family: var(--mono); font-size: 12.5px; text-align: right; }
  .foot { display: flex; justify-content: flex-end; font-size: 13px; color: var(--muted); }
  @keyframes breathe { 0%, 100% { opacity: .3; } 50% { opacity: 1; } }
</style>
