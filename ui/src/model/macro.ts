import type { MacroStep, ApplicationTarget, KeyEntry } from "./keymap";
import { entriesLabel, switchTo } from "./labels";
import { pick } from "../lib/i18n";

// 宏编辑器的纯逻辑：步骤标签、校验（与 Rust Macro::validate 一致）、不可变的增删移。
// 校验信息的中文与 haloProblems 一样是数据字符串，第 5 期做结构化 i18n。

export type MacroStepKind = "keyboard" | "application" | "delay";
export const stepKind = (s: MacroStep): MacroStepKind => Object.keys(s)[0] as MacroStepKind;

export function stepLabel(step: MacroStep): string {
  if ("keyboard" in step) return entriesLabel((step as { keyboard: { _0: KeyEntry[] } }).keyboard._0);
  if ("application" in step) return switchTo((step as { application: { _0: ApplicationTarget } }).application._0.name);
  const s = (step as { delay: { _0: number } }).delay._0;
  return pick({ zh: `等待 ${s} 秒`, en: `Wait ${s} s`, ja: `${s} 秒待つ` });
}

export function macroProblems(name: string, steps: MacroStep[]): string[] {
  const p: string[] = [];
  const n = name.trim();
  if (!n) p.push("名称不能为空");
  else if ([...n].length > 80) p.push("名称不能超过 80 字");
  if (!steps.length) p.push("至少要有一步");
  if (steps.length > 32) { p.push("最多 32 步"); return p; }
  steps.forEach((s, i) => {
    if ("delay" in s) {
      const d = (s as { delay: { _0: number } }).delay._0;
      if (!Number.isFinite(d) || d < 0.05 || d > 10) p.push(`第 ${i + 1} 步等待时间应为 0.05–10 秒`);
    }
    if ("keyboard" in s && !(s as { keyboard: { _0: KeyEntry[] } }).keyboard._0.length) p.push(`第 ${i + 1} 步没有按键`);
  });
  return p;
}

/** 交换第 i 步与相邻步；越界时原样返回同一个数组。 */
export function moveStep(steps: MacroStep[], i: number, delta: -1 | 1): MacroStep[] {
  const j = i + delta;
  if (j < 0 || j >= steps.length) return steps;
  const out = steps.slice();
  [out[i], out[j]] = [out[j]!, out[i]!];
  return out;
}
export const removeStep = (steps: MacroStep[], i: number): MacroStep[] => steps.filter((_, k) => k !== i);
export const insertStep = (steps: MacroStep[], at: number, step: MacroStep): MacroStep[] => [...steps.slice(0, at), step, ...steps.slice(at)];
