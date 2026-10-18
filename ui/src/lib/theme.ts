// 外观：深色（默认）/ 浅色 / 跟随系统。只是本机偏好，和语言一样存 localStorage。
export type Theme = "dark" | "light" | "system";

export function resolveTheme(saved: string | null | undefined): Theme {
  return saved === "dark" || saved === "light" || saved === "system" ? saved : "dark";
}

let media: MediaQueryList | null = null;
const syncSystem = () => {
  const root = document.documentElement;
  root.classList.toggle("light-system", root.dataset.theme === "system" && !!media?.matches);
};

/** 把选择写到 <html data-theme>；"system" 时再按系统亮暗加 .light-system（app.css 据此选浅色 token）。 */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.dataset.theme = theme;
  if (!media && typeof window !== "undefined" && typeof window.matchMedia === "function") {
    media = window.matchMedia("(prefers-color-scheme: light)");
    media.addEventListener("change", syncSystem);
  }
  syncSystem();
}

export function getTheme(): Theme {
  try { return resolveTheme(localStorage.getItem("halokey.theme")); } catch { return "dark"; }
}

export function setTheme(theme: Theme) {
  try { localStorage.setItem("halokey.theme", theme); } catch { /* 忽略 */ }
  applyTheme(theme);
}
