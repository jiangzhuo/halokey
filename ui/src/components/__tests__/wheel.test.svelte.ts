import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import HaloWheel from "../HaloWheel.svelte";
import { setLang } from "../../lib/i18n";

let app: ReturnType<typeof mount> | undefined;
afterEach(() => { if (app) unmount(app); app = undefined; document.body.innerHTML = ""; });
const kb = (c: number) => ({ keyboard: { _0: [{ type: 2, code: c }] } });

describe("halo wheel", () => {
  it("draws one wedge per option, highlights the selection and names it in the centre", () => {
    setLang("zh");
    const halo = { id: "m", name: "主菜单", options: [{ label: "甲", action: kb(4) }, { label: "乙", action: kb(5) }, { label: "丙", action: kb(6) }] };
    app = mount(HaloWheel, { target: document.body, props: { halo, selected: 1, size: 360 } });
    flushSync();
    expect(document.querySelectorAll("path.wedge")).toHaveLength(3);
    expect(document.querySelector("path.wedge.sel")!.getAttribute("data-index")).toBe("1");
    expect(document.querySelector(".center")!.textContent).toContain("乙");
    expect(document.body.textContent).toContain("拧旋钮选择");
    const svg = document.querySelector("svg")!;
    expect(svg.getAttribute("width")).toBe("360");
  });
  it("renders nothing without a halo", () => {
    app = mount(HaloWheel, { target: document.body, props: { halo: null, selected: 0, size: 280 } });
    flushSync();
    expect(document.querySelectorAll("path.wedge")).toHaveLength(0);
  });
});
