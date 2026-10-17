import type { Halo, HostAction } from "./keymap";

/** 编辑器里的一个槽位；`action` 为 null 表示空槽（不写进配置）。 */
export type Slot = { label: string; action: HostAction | null };
export const SLOT_COUNT = 8;
const empty = (): Slot => ({ label: "", action: null });

export function slotsFrom(halo: Halo | null): Slot[] {
  const slots = Array.from({ length: SLOT_COUNT }, empty);
  halo?.options.slice(0, SLOT_COUNT).forEach((o, i) => { slots[i] = { label: o.label, action: JSON.parse(JSON.stringify(o.action)) }; });
  return slots;
}

export function haloFrom(id: string, name: string, slots: Slot[]): Halo {
  return { id, name: name.trim(), options: slots.filter((s) => s.action).map((s) => ({ label: s.label.trim(), action: s.action! })) };
}

export function haloProblems(name: string, slots: Slot[]): string[] {
  const problems: string[] = [];
  const n = name.trim();
  if (!n) problems.push("名称不能为空");
  else if ([...n].length > 80) problems.push("名称不能超过 80 字");
  if (!slots.some((s) => s.action)) problems.push("至少要有一个选项");
  slots.forEach((s, i) => {
    if (!s.action) return;
    if (!s.label.trim()) problems.push(`槽位 ${i + 1} 有动作但没有名字`);
    else if ([...s.label.trim()].length > 80) problems.push(`槽位 ${i + 1} 的名字不能超过 80 字`);
  });
  return problems;
}

export function moveSlot(slots: Slot[], from: number, delta: -1 | 1): Slot[] {
  const to = from + delta;
  if (to < 0 || to >= slots.length) return slots;
  const out = slots.map((s) => ({ ...s }));
  [out[from], out[to]] = [out[to]!, out[from]!];
  return out;
}

export function clearSlot(slots: Slot[], i: number): Slot[] {
  const out = slots.map((s) => ({ ...s }));
  out[i] = empty();
  return out;
}
