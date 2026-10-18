import { describe, it, expect, afterEach } from "vitest";
import { resolveTheme, applyTheme, setTheme, getTheme } from "../theme";

afterEach(() => { document.documentElement.removeAttribute("data-theme"); document.documentElement.classList.remove("light-system"); try { localStorage.removeItem("halokey.theme"); } catch { /* */ } });

describe("theme", () => {
  it("defaults to dark unless a valid preference was saved", () => {
    expect(resolveTheme(null)).toBe("dark");
    expect(resolveTheme("blue")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("system")).toBe("system");
  });
  it("applies the choice to the root element and remembers it", () => {
    applyTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    applyTheme("system");
    expect(document.documentElement.dataset.theme).toBe("system");
    setTheme("dark");
    expect(getTheme()).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});
