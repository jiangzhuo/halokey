<script lang="ts">
  import { t } from "../lib/i18n";
  import { usageFromCode, isModifier, comboEntries, MODIFIER_USAGES } from "../model/hidcodes";
  import { CATEGORIES } from "../model/codes";
  import { keyName, entriesLabel } from "../model/labels";
  import type { KeyEntry } from "../model/keymap";

  /** 录制框：聚焦后按真实键盘，修饰键累加、主键替换；也可手动勾修饰键 + 选主键。 */
  let { onAssign, assignLabel }: { onAssign: (entries: KeyEntry[]) => void; assignLabel?: string } = $props();
  let mods = $state<Set<number>>(new Set());
  let main = $state<number | null>(null);
  let hint = $state("");
  const entries = $derived(comboEntries(mods, main));
  const MAIN_KEYS = CATEGORIES.filter((c) => ["common", "letters", "fkeys", "keypad"].includes(c.id))
    .flatMap((c) => c.keycaps ?? [])
    .filter((k) => !isModifier(k.entry.code) && k.entry.code > 1);

  function onKey(e: KeyboardEvent) {
    e.preventDefault();
    if (e.code === "Escape") { mods = new Set(); main = null; hint = ""; return; }
    const usage = usageFromCode(e.code);
    if (usage === null) { hint = t("不支持这个键"); return; }
    hint = "";
    if (isModifier(usage)) { const s = new Set(mods); s.add(usage); mods = s; } else main = usage;
  }
  function toggleMod(u: number) { const s = new Set(mods); if (s.has(u)) s.delete(u); else s.add(u); mods = s; }
  function assign() { if (!entries.length) return; onAssign(entries); mods = new Set(); main = null; hint = ""; }
</script>

<div class="combo">
  <button class="rec" data-role="combo-rec" onkeydown={onKey}>
    <span class="preview">{entries.length ? entriesLabel(entries) : t("点这里，然后按下组合键")}</span>
    <small class:bad={!!hint}>{hint || t("录制：先按修饰键再按主键；Esc 清空")}</small>
  </button>
  <div class="manual">
    {#each MODIFIER_USAGES as u}
      <label><input type="checkbox" data-mod={u} checked={mods.has(u)} onchange={() => toggleMod(u)} /> {keyName({ type: 2, code: u })}</label>
    {/each}
    <select data-role="combo-main" value={main === null ? "" : String(main)} onchange={(e) => { const v = (e.target as HTMLSelectElement).value; main = v === "" ? null : Number(v); }}>
      <option value="">{t("主键…")}</option>
      {#each MAIN_KEYS as k}<option value={String(k.entry.code)}>{k.label}</option>{/each}
    </select>
    <button class="primary" data-role="combo-assign" disabled={!entries.length} onclick={assign}>{assignLabel ?? t("分配组合键")}</button>
  </div>
</div>

<style>
  .combo { display: flex; flex-direction: column; gap: 12px; }
  .rec { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 18px; border: 1px dashed var(--line-strong); border-radius: var(--r-lg); background: var(--card); min-height: 80px; }
  .rec:hover { background: var(--card); border-color: var(--muted); }
  .rec:focus { outline: 2px solid var(--accent); border-color: transparent; }
  .preview { font-size: 18px; font-weight: 600; }
  .rec small { color: var(--muted); font-size: 12px; }
  .rec small.bad { color: var(--bad); }
  .manual { display: flex; flex-wrap: wrap; gap: 10px 14px; align-items: center; font-size: 13px; }
  .manual label { display: inline-flex; align-items: center; gap: 4px; }
</style>
