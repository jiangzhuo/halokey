<script lang="ts">
  import { t } from "../lib/i18n";
  import RingPreview from "../components/RingPreview.svelte";
  import Palette from "../components/Palette.svelte";
  import { actionLabel } from "../model/labels";
  import { slotsFrom, haloFrom, haloProblems, moveSlot, clearSlot, type Slot } from "../model/halo";
  import { newHaloId, upsertHalo, deleteHalo, haloActionAllowed, type Draft } from "../model/draft";
  import type { HostAction } from "../model/keymap";

  let { draft = $bindable(), haloId, onClose }: { draft: Draft; haloId: string | null; onClose: (savedId: string | null) => void } = $props();
  // 进入编辑器时定下 id 与初值（App 每次打开都重新挂载本组件）；之后只改本地状态，保存时一次写回草稿。
  // svelte-ignore state_referenced_locally
  const existing = haloId ? (draft.map?.halos ?? []).find((h) => h.id === haloId) ?? null : null;
  // svelte-ignore state_referenced_locally
  const id = haloId ?? newHaloId(draft.map!);
  let name = $state(existing?.name ?? "");
  let slots = $state<Slot[]>(slotsFrom(existing));
  let selected = $state(0);
  let problems = $state<string[]>([]);
  let blocked = $state<string[]>([]);
  const halos = $derived(draft.map?.halos ?? []);

  function pickAction(a: HostAction) {
    if (!haloActionAllowed(a, id)) { problems = [t("光环选项不能引用功能库、切层或自己")]; return; }
    const s = slots.map((x) => ({ ...x }));
    s[selected] = { label: s[selected]!.label, action: a };
    slots = s; problems = [];
  }
  function setLabel(i: number, v: string) {
    const c = slots.map((x) => ({ ...x }));
    c[i] = { ...c[i]!, label: v };
    slots = c;
  }
  function save() {
    problems = haloProblems(name, slots);
    if (problems.length) return;
    draft = upsertHalo(draft, haloFrom(id, name, slots));
    onClose(id);
  }
  function remove() {
    const r = deleteHalo(draft, id);
    if (r.blockedBy.length) { blocked = r.blockedBy; return; }
    draft = r.draft; onClose(null);
  }
</script>

<div class="page">
  <header>
    <button class="back" onclick={() => onClose(null)} title={t("返回")}>←</button>
    <h2>{existing ? t("编辑光环") : t("新建光环")}</h2>
    <input data-role="halo-name" placeholder={t("光环名称")} bind:value={name} />
    <span class="id mono">{id}</span>
  </header>
  <section class="body">
    <RingPreview {slots} {selected} {halos} onPick={(i) => (selected = i)} />
    <div class="slots">
      {#each slots as s, i}
        <div class="slot" class:on={i === selected} role="button" tabindex="0" onclick={() => (selected = i)} onkeydown={(e) => e.key === "Enter" && (selected = i)}>
          <span class="n mono">{i + 1}</span>
          <input data-role="slot-label" placeholder={t("名称")} value={s.label} oninput={(e) => setLabel(i, (e.target as HTMLInputElement).value)} />
          <span class="act">{s.action ? actionLabel(s.action, null, halos) : t("（空）")}</span>
          <span class="ops">
            <button data-role="slot-up" disabled={i === 0} onclick={(e) => { e.stopPropagation(); slots = moveSlot(slots, i, -1); selected = Math.max(0, i - 1); }}>↑</button>
            <button data-role="slot-down" disabled={i === slots.length - 1} onclick={(e) => { e.stopPropagation(); slots = moveSlot(slots, i, 1); selected = Math.min(slots.length - 1, i + 1); }}>↓</button>
            <button data-role="slot-clear" disabled={!s.action && !s.label} onclick={(e) => { e.stopPropagation(); slots = clearSlot(slots, i); }}>✕</button>
          </span>
        </div>
      {/each}
    </div>
  </section>
  <Palette bind:draft {halos} apps={(draft.map?.actionLibrary ?? []).filter((a) => "application" in a.action)} macros={(draft.map?.actionLibrary ?? []).filter((a) => "macro" in a.action)} mode="action" onAction={pickAction} excludeHalo={id} crumb={`${t("槽位")} ${selected + 1}`} />
  <footer>
    {#if problems.length}<span class="bad">{problems.join("；")}</span>{/if}
    {#if blocked.length}<span class="bad">{t("被以下位置引用，无法删除")}：{blocked.join("、")}</span>{/if}
    {#if existing}<button class="danger" onclick={remove}>{t("删除光环")}</button>{/if}
    <button class="ghost" onclick={() => onClose(null)}>{t("取消")}</button>
    <button class="primary" onclick={save}>{t("保存光环")}</button>
  </footer>
</div>

<style>
  .page { display: flex; flex-direction: column; min-height: 100%; }
  header { display: flex; align-items: center; gap: 14px; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  header h2 { margin: 0; font-size: 16px; }
  header input { flex: 0 0 260px; }
  .back { background: none; font-size: 18px; padding: 2px 8px; }
  .back:hover { background: var(--card); }
  .id { color: var(--muted); font-size: 12px; }
  .mono { font-family: var(--mono); }
  .body { display: grid; grid-template-columns: 420px 1fr; gap: 24px; padding: 16px 22px; align-items: center; }
  .slots { display: flex; flex-direction: column; gap: 6px; }
  .slot { display: grid; grid-template-columns: 24px 1fr 1.2fr auto; gap: 10px; align-items: center; background: var(--card); border-radius: var(--r-lg); padding: 6px 12px; border: 1px solid transparent; cursor: pointer; }
  .slot.on { border-color: var(--accent); }
  .slot .n { color: var(--muted); }
  .slot .act { color: var(--accent); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .slot .ops { display: flex; gap: 4px; }
  .slot .ops button { padding: 2px 8px; }
  footer { display: flex; justify-content: flex-end; align-items: center; gap: 10px; padding: 12px 22px; border-top: 1px solid var(--line); position: sticky; bottom: 0; background: var(--bg); }
  .bad { color: var(--bad); margin-right: auto; font-size: 13px; }
</style>
