import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

// 正式 CSP 是 style-src 'self' / script-src 'self'：入口 HTML 里不能有内联样式属性或内联脚本。
describe("entry HTML respects the strict CSP", () => {
  for (const f of ["index.html", "halo.html"]) {
    it(`${f} has no inline style attributes or inline scripts`, () => {
      const html = readFileSync(resolve(__dirname, "../../..", f), "utf8");
      expect(html).not.toMatch(/\sstyle=/);
      expect(html).not.toMatch(/<script(?![^>]*\ssrc=)[^>]*>[^<]*\S[^<]*<\/script>/);
    });
  }
});
