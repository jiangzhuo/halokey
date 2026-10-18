import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { EN, JA, t, pick, setLang, resolveLang } from "../i18n";

function walk(dir: string, out: string[] = []): string[] {
  for (const f of readdirSync(dir)) { const p = join(dir, f); if (statSync(p).isDirectory()) walk(p, out); else if (/\.(ts|svelte)$/.test(f) && !p.includes("__tests__")) out.push(p); }
  return out;
}

describe("i18n", () => {
  it("has an English entry for every t(\"…\") literal in the sources", () => {
    const missing = new Set<string>();
    for (const file of walk(join(__dirname, "../.."))) {
      const src = readFileSync(file, "utf8");
      for (const m of src.matchAll(/\bt\("([^"\\]+)"\)/g)) if (!(m[1]! in EN)) missing.add(m[1]!);
    }
    expect([...missing]).toEqual([]);
  });
  it("falls back to Chinese", () => {
    setLang("en"); expect(t("键位")).toBe("Keymap"); expect(t("没有这条")).toBe("没有这条");
    setLang("zh"); expect(t("键位")).toBe("键位");
  });
  it("has a Japanese entry for every English one, and speaks Japanese when selected", () => {
    expect(Object.keys(JA).sort()).toEqual(Object.keys(EN).sort());
    for (const k of Object.keys(JA)) expect(JA[k], k).not.toBe("");
    setLang("ja");
    try {
      expect(t("键位")).toBe("キーマップ");
      expect(t("设置")).toBe("設定");
      expect(t("没有这条")).toBe("没有这条");
      expect(pick({ zh: "宏", en: "Macro", ja: "マクロ" })).toBe("マクロ");
    } finally { setLang("zh"); }
    setLang("en"); try { expect(pick({ zh: "宏", en: "Macro", ja: "マクロ" })).toBe("Macro"); } finally { setLang("zh"); }
    expect(pick({ zh: "宏", en: "Macro", ja: "マクロ" })).toBe("宏");
  });
  it("defaults to English unless a valid preference was saved", () => {
    expect(resolveLang(null)).toBe("en");
    expect(resolveLang("")).toBe("en");
    expect(resolveLang("fr")).toBe("en");
    expect(resolveLang("zh")).toBe("zh");
    expect(resolveLang("ja")).toBe("ja");
  });
  it("has no Chinese literal left in .svelte markup outside t()", () => {
    const offenders: string[] = [];
    for (const file of walk(join(__dirname, "../.."))) {
      if (!file.endsWith(".svelte")) continue;
      const markup = readFileSync(file, "utf8").split("\n").filter((l) => !l.includes("i18n-ignore")).join("\n").replace(/<script[\s\S]*?<\/script>/g, "").replace(/<style[\s\S]*?<\/style>/g, "").replace(/\bt\("[^"]+"\)/g, "").replace(/<!--[\s\S]*?-->/g, "");
      for (const line of markup.split("\n")) if (/[\u4e00-\u9fff]/.test(line)) offenders.push(`${file.split("/src/")[1]}: ${line.trim()}`);
    }
    expect(offenders).toEqual([]);
  });
});
