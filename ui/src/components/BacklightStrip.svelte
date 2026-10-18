<script lang="ts">
  import { t } from "../lib/i18n";
  import { LIGHT_NAMES } from "../model/labels";
  import type { LayerLighting, LightEffect, LightMode } from "../model/keymap";
  let { layer, lighting, own, onEffect, onFollow, onClearBase }: { layer: number; lighting: LayerLighting | null; own: boolean; onEffect: (light: number, patch: Partial<LightEffect>) => void; onFollow: (follow: boolean) => void; onClearBase: () => void } = $props();
  let expanded = $state(false);
  let wrap = $state<HTMLDivElement | null>(null);
  // 展开的编辑器是浮层：点外面或按 Esc 关闭。
  $effect(() => {
    if (!expanded) return;
    const onDown = (e: MouseEvent) => { if (wrap && !wrap.contains(e.target as Node)) expanded = false; };
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") expanded = false; };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("mousedown", onDown); window.removeEventListener("keydown", onKey); };
  });
  const MODES: LightMode[] = ["off", "solid", "breathe"];
  const modeName: Record<LightMode, string> = { off: "关", solid: "常亮", breathe: "呼吸" };
  const summary = $derived(!lighting ? (layer === 0 ? t("未设置（不管灯）") : t("跟随 Layer 0")) : "");
  // 状态条里每盏灯一个紧凑小块：短名 + 模式圆点 + 亮度（关着的不显示亮度）。
  const SHORT = ["1", "2", "3", "⟳"];
  // AU05 的键 1（🎙）没有可见的灯（2026-10-18 实测：直接写设备也不亮）；记录里仍保留四盏以兼容格式。
  const INVISIBLE = new Set([0]);
</script>

<div class="wrap" bind:this={wrap}>
  <div class="strip">
    <span class="title">{t("本层背光")}</span>
    {#if lighting}
      {#each lighting.lights as l, i}
        {#if !INVISIBLE.has(i)}<span class="lamp" title={`${t(LIGHT_NAMES[i]!)} · ${t(modeName[l.mode])} · ${l.brightness}`}><b>{SHORT[i]}</b><span class="dot" data-mode={l.mode}></span>{#if l.mode !== "off"}<span class="mono">{l.brightness}</span>{/if}</span>{/if}
      {/each}
    {:else}
      <span class="sum">{summary}</span>
    {/if}
    {#if layer !== 0 && !own}<span class="muted">▽</span>{/if}
    <button class="toggle" class:open={expanded} aria-expanded={expanded} title={expanded ? t("收起") : t("编辑")} onclick={() => (expanded = !expanded)}>{t("编辑")}<i class="chev" aria-hidden="true">⌄</i></button>
  </div>
  {#if expanded}
    <div class="editor">
      {#each [0, 1, 2, 3] as i}
        {@const l = lighting?.lights[i] ?? { mode: "off", brightness: 0 }}
        {@const invisible = INVISIBLE.has(i)}
        <div class="light" class:invisible>
          <span class="name">{t(LIGHT_NAMES[i]!)}</span>
          {#if invisible}
            <span class="note">{t("此设备无可见灯")}</span><span></span><span></span>
          {:else}
            <span class="seg">
              {#each MODES as m}<button data-mode={m} class:on={l.mode === m} onclick={() => onEffect(i, m === "off" ? { mode: m } : { mode: m, brightness: l.brightness || 20 })}>{t(modeName[m])}</button>{/each}
            </span>
            <input type="range" min="0" max="20" value={l.brightness} disabled={l.mode === "off"} oninput={(e) => onEffect(i, { brightness: Number((e.target as HTMLInputElement).value) })} />
            <span class="mono">{l.brightness}</span>
          {/if}
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
  .wrap { position: relative; }
  .strip { display: flex; align-items: center; gap: 8px; background: var(--card); border-radius: var(--r-md); padding: 6px 10px; font-size: 13px; }
  .title { color: var(--muted); }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--line-strong); display: inline-block; }
  .dot[data-mode="solid"] { background: var(--warn); }
  .dot[data-mode="breathe"] { background: var(--warn); animation: breathe 1.8s ease-in-out infinite; }
  .sum { max-width: 360px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lamp { display: inline-flex; align-items: center; gap: 4px; padding: 2px 6px; border-radius: var(--r-sm); background: var(--bg); }
  .lamp b { font-weight: 600; font-size: 12px; color: var(--muted); }
  .lamp .mono { font-family: var(--mono); font-size: 12px; }
  .muted { color: var(--muted); }
  /* 固定宽度的切换按钮：文字不变，只转箭头，展开 / 收起时条不伸缩；描边让它看起来可点。 */
  .strip .toggle { padding: 3px 8px 3px 10px; font-size: 13px; background: var(--card-on); border: 1px solid var(--line-strong); display: inline-flex; align-items: center; gap: 6px; min-width: 72px; justify-content: center; }
  .strip .toggle:hover { border-color: var(--accent); }
  .strip .toggle.open { border-color: var(--accent); color: var(--accent); }
  .chev { font-style: normal; display: inline-block; transition: transform 150ms; line-height: 1; }
  .toggle.open .chev { transform: rotate(180deg); }
  /* 浮层：挂在状态条下方、右对齐，不把下面的内容挤走。 */
  .editor { position: absolute; top: calc(100% + 6px); right: 0; z-index: 30; background: var(--surface); border: 1px solid var(--line); border-radius: var(--r-lg); padding: 10px 14px; display: flex; flex-direction: column; gap: 8px; min-width: 460px; box-shadow: var(--shadow-pop); }
  .light { display: grid; grid-template-columns: 48px auto 1fr 34px; align-items: center; gap: 10px; }
  .name { color: var(--muted); font-size: 13px; }
  .seg { display: inline-flex; background: var(--card); border-radius: var(--r-md); padding: 3px; gap: 2px; }
  .seg button { background: none; padding: 4px 10px; font-size: 13px; }
  .seg button:hover { background: var(--card-on); }
  .seg button.on { background: var(--card-on); color: var(--accent); }
  .mono { font-family: var(--mono); font-size: 12px; text-align: center; font-variant-numeric: tabular-nums; background: var(--bg); border-radius: var(--r-sm); padding: 1px 4px; }
  .light.invisible .name { opacity: .55; }
  .note { color: var(--muted); font-size: 12px; }
  .foot { display: flex; justify-content: flex-end; font-size: 13px; color: var(--muted); }
  @keyframes breathe { 0%, 100% { opacity: .3; } 50% { opacity: 1; } }
</style>
