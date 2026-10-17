# 设置界面重做 · 第 1 期实施计划（前端骨架 + 键位页 + 设置页只读）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 Svelte 5 + Vite + TypeScript 重建 halokey 桌面应用的前端，交付 olanzi 风格的键位页（Layer 条、设备渲染图、手势卡片、分类键码面板、撤销 / 保存）和只读的设置页，替换现有的原生 JS 调试面板。

**Architecture:** 前端是纯展示层：键位编辑在内存草稿（纯函数 reducer）里完成，保存时经 Rust 校验后写文件并下发给设备线程；所有设备交互仍走现有 `halokey-runtime` 的命令 / 快照 / 事件。Vite 多页构建出 `index.html`（主窗口）和 `halo.html`（转盘），Tauri 把 `ui/dist` 打进二进制。

**Tech Stack:** Svelte 5（runes）、Vite 8、TypeScript 5.9、vitest 5、@tauri-apps/api 2.12、Tauri 2.12、Rust workspace 现有 crate。

**Spec:** `docs/superpowers/specs/2026-10-17-settings-ui-redesign-design.md`（本计划只覆盖其 §12 第 1 期；第 2–6 期各自另写计划）。

## Global Constraints

- 窗口最小 960×640，默认 1100×760；深色为默认主题，所有颜色走 `:root` CSS 变量。
- 界面文字以中文为源文本，英文经 `t()` 查表；新增字符串必须同时加进英文表。
- 配置 JSON 形状与 Rust `HostKeymap` 完全一致（olanzi 的 `{"kind": {"_0": …}}` 枚举编码），前端不得改写版本号，版本由 Rust 决定。
- 前端永远不直接碰设备：读写都通过 Tauri 命令；没有 Tauri 环境（`npm run dev` 在浏览器里）时用 `lib/tauri.ts` 的 mock。
- 序列号不显示在界面上。
- 每个任务结束：`npm test`、`npm run check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 全绿，再提交；提交信息英文，末尾带 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- 不新增 Rust 依赖；前端只允许 §Task 1 列出的依赖。

## Review Focus

1. **配置文件里出现前端不认识的动作类型**（比如将来加的 `shell`）：键位页必须仍能打开，该手势显示为"复杂动作"，保存时原样保留，不得丢字段。→ Task 2 的往返测试覆盖未知动作。
2. **草稿与设备线程的键位不一致**（设备重连后 `get_keymap` 返回了新内容）：有未保存改动时不能被静默覆盖。→ Task 4 的 reducer 测试 `loadSaved` 在 dirty 时不覆盖。
3. **旋转手势被分配了不允许的东西**（双击 / 长按 / 保持按住 / MO）：面板必须禁用这些选项，保存时 Rust 校验是最后一道门。→ Task 4、Task 8 测试。
4. **接收器断开时点保存**：文件照样写，`SetKeymap` 在线程里排队，重连后生效；界面不能报"保存失败"。→ Task 9 的 mock 测试与真机检查。
5. **英文界面漏翻**：新字符串全部进 `t()`。→ Task 6 的 i18n 完整性测试扫描源码里的 `t("…")` 字面量。

---

## 文件结构

```
ui/
  package.json  vite.config.ts  tsconfig.json  svelte.config.js  vitest.config.ts
  index.html  halo.html
  src/
    main.ts                 主窗口入口
    halo.ts                 转盘窗口入口（第 1 期保留现在的圆泡实现，第 3 期换扇形环）
    app.css                 主题变量与基础样式
    App.svelte              顶栏 + 页面切换
    lib/tauri.ts            Tauri 命令 / 事件的类型化封装 + 浏览器 mock
    lib/i18n.ts             t() 与英文表（从旧 i18n.js 迁移）
    model/keymap.ts         HostKeymap / HostAction / KeyEntry 等类型与守卫
    model/labels.ts         控件名、键名、动作简名
    model/codes.ts          键码面板的分类与键帽
    model/draft.ts          键位草稿 reducer（选中、分配、输出方式、撤销）
    components/TopBar.svelte  LayerBar.svelte  DeviceView.svelte  GestureCard.svelte
               KnobLabel.svelte  Palette.svelte  KeyGrid.svelte  OutputModeSegment.svelte
               BottomBar.svelte  HaloBubbles.svelte
    pages/Keymap.svelte  pages/Settings.svelte
    model/__tests__/keymap.test.ts  labels.test.ts  codes.test.ts  draft.test.ts
    lib/__tests__/tauri.test.ts  i18n.test.ts
apps/halokey/tauri.conf.json         build 钩子、devUrl、frontendDist、devCsp
apps/halokey/src/main.rs             新增 validate_keymap 命令；UiSnapshot 增加 activeLayers
crates/runtime/src/lib.rs            Snapshot 增加 active_layers，apply() 后刷新
.github/workflows/ci.yml             cargo 之前先 npm ci && npm run build
```

旧的 `ui/*.js`、`ui/*.css`、旧 `ui/index.html`、`ui/halo.html` 在 Task 11 删除；Task 1 先把 Vite 入口放在同名路径之外（`ui/index.html` 会被覆盖，先在 Task 1 里备份到 `ui/legacy/`）。

---

### Task 1: 前端工程骨架与 Tauri 接线

**Files:**
- Create: `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/svelte.config.js`, `ui/vitest.config.ts`, `ui/src/main.ts`, `ui/src/halo.ts`, `ui/src/App.svelte`, `ui/src/app.css`, `ui/src/components/HaloBubbles.svelte`
- Move: `ui/index.html ui/halo.html ui/app.js ui/editor.js ui/halo.js ui/i18n.js ui/style.css ui/halo.css` → `ui/legacy/`（Task 11 删除）
- Create: `ui/index.html`, `ui/halo.html`（Vite 入口）
- Modify: `apps/halokey/tauri.conf.json`, `.gitignore`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces: `npm run dev`（Vite 5173）、`npm run build`（`ui/dist/index.html` + `ui/dist/halo.html`）、`npm test`、`npm run check`；Tauri 的 `beforeDevCommand` / `beforeBuildCommand` 指向它们。

- [ ] **Step 1: 备份旧前端**

```bash
cd ~/Desktop/halokey && mkdir -p ui/legacy && git mv ui/index.html ui/halo.html ui/app.js ui/editor.js ui/halo.js ui/i18n.js ui/style.css ui/halo.css ui/legacy/
```

- [ ] **Step 2: 写 package.json 与工具配置**

`ui/package.json`：

```json
{
  "name": "halokey-ui",
  "private": true,
  "version": "0.0.1",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^7.3.1",
    "@tauri-apps/api": "^2.12.4",
    "@tsconfig/svelte": "^5.0.8",
    "jsdom": "^30.1.3",
    "svelte": "^5.57.2",
    "svelte-check": "^4.7.7",
    "typescript": "~5.9.3",
    "vite": "^8.3.4",
    "vitest": "^5.0.5"
  }
}
```

`ui/vite.config.ts`：

```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

// 两个入口：主窗口与转盘悬浮窗。Tauri 用 ui/dist 作为 frontendDist。
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5173, strictPort: true, fs: { allow: [".."] } }, // lib/tauri.ts 的 mock 会 import ../docs/examples
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    rollupOptions: {
      input: { main: resolve(__dirname, "index.html"), halo: resolve(__dirname, "halo.html") },
    },
  },
});
```

`ui/vitest.config.ts`：

```ts
import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte({ hot: false })],
  test: { environment: "jsdom", include: ["src/**/*.test.ts"] },
});
```

`ui/svelte.config.js`：

```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
export default { preprocess: vitePreprocess() };
```

`ui/tsconfig.json`：

```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": { "target": "ES2022", "module": "ESNext", "moduleResolution": "bundler", "strict": true, "noUncheckedIndexedAccess": true, "resolveJsonModule": true, "types": ["vite/client"] },
  "include": ["src/**/*.ts", "src/**/*.svelte", "vite.config.ts", "vitest.config.ts"]
}
```

- [ ] **Step 3: 写入口文件与最小 App**

`ui/index.html`：

```html
<!doctype html>
<html lang="zh">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>halokey</title>
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

`ui/halo.html` 同上，`<title>halo</title>`，`src="/src/halo.ts"`，`<body style="background:transparent">`。

`ui/src/main.ts`：

```ts
import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
```

`ui/src/halo.ts`：

```ts
import { mount } from "svelte";
import HaloBubbles from "./components/HaloBubbles.svelte";

mount(HaloBubbles, { target: document.body });
```

`ui/src/App.svelte`（第 1 期临时内容，Task 6 替换）：

```svelte
<main><h1>halokey</h1><p>前端骨架已接通。</p></main>
```

`ui/src/app.css`：

```css
:root {
  --bg: #1c1c1e; --panel: #232326; --card: #2a2a2d; --card-on: #3a3a3f; --line: #2c2c30;
  --text: #e8e8ea; --muted: #9a9aa2; --accent: #f2c9b5; --accent-text: #1c1c1e;
  --ok: #7ee787; --bad: #ff7b72; --warn: #fbbf24;
  --radius: 10px; --font: 14px/1.5 -apple-system, "Segoe UI", "Noto Sans CJK SC", "PingFang SC", system-ui, sans-serif;
  --mono: ui-monospace, "JetBrains Mono", Menlo, monospace;
}
:root[data-theme="light"] {
  --bg: #f5f5f7; --panel: #ffffff; --card: #ececf0; --card-on: #d9d9e0; --line: #e1e1e6;
  --text: #1c1c1e; --muted: #6e6e76; --accent: #e8a98a; --accent-text: #1c1c1e;
}
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; background: var(--bg); color: var(--text); font: var(--font); }
button { font: inherit; color: inherit; background: var(--card); border: 0; border-radius: 8px; padding: 7px 14px; cursor: pointer; }
button:hover { filter: brightness(1.12); }
button.primary { background: var(--accent); color: var(--accent-text); font-weight: 600; }
button:disabled { opacity: .45; cursor: default; filter: none; }
input, select { font: inherit; color: var(--text); background: var(--bg); border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; }
```

`ui/src/components/HaloBubbles.svelte`（移植旧 halo.js，第 3 期替换为扇形环）：

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { subscribe } from "../lib/tauri";
  type Opt = { label: string };
  let name = $state("");
  let options = $state<Opt[]>([]);
  let selected = $state(0);
  const R = 150, C = 210;
  onMount(() => {
    let un: (() => void) | undefined;
    subscribe((e) => {
      if (e.kind === "haloShow") { name = e.halo.name; options = e.halo.options; selected = e.selected; }
      else if (e.kind === "haloSelect") selected = e.selected;
    }).then((u) => { un = u; });
    return () => un?.();
  });
  // 用 style: 指令（CSSOM）而不是 style 属性：正式 CSP 没有 'unsafe-inline'，setAttribute("style") 会被拦。
  const angle = (i: number) => -Math.PI / 2 + (i * 2 * Math.PI) / Math.max(options.length, 1);
  const px = (v: number) => `${v}px`;
</script>

<div class="ring">
  <div class="center">{name}</div>
  {#each options as o, i}
    <div class="opt" class:sel={i === selected} style:left={px(C + R * Math.cos(angle(i)))} style:top={px(C + R * Math.sin(angle(i)))}>{o.label}</div>
  {/each}
</div>
<div class="hint">拧旋钮选择 · 按下确认 · 按任意键取消</div>

<style>
  :global(html, body) { margin: 0; background: transparent; overflow: hidden; width: 420px; height: 420px; }
  .ring { position: relative; width: 420px; height: 420px; font: 13px/1.3 system-ui, sans-serif; color: #e6e9f0; }
  .center { position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 118px; height: 118px; border-radius: 50%; background: rgba(17,20,28,.92); border: 2px solid rgba(242,201,181,.6); display: flex; align-items: center; justify-content: center; text-align: center; font-weight: 600; }
  .opt { position: absolute; width: 96px; height: 96px; margin: -48px 0 0 -48px; border-radius: 50%; background: rgba(27,31,42,.92); border: 2px solid rgba(42,48,64,.9); display: flex; align-items: center; justify-content: center; text-align: center; padding: 8px; transition: transform 90ms, background 90ms; }
  .opt.sel { background: #f2c9b5; color: #0b1220; border-color: #f2c9b5; transform: scale(1.14); font-weight: 700; }
  .hint { position: absolute; left: 0; right: 0; bottom: 6px; text-align: center; color: rgba(230,233,240,.55); font-size: 11px; }
</style>
```

`subscribe` 在 Task 5 定义；本任务先放一个最小版本到 `ui/src/lib/tauri.ts` 让骨架能编译：

```ts
export type UiEvent = { kind: string } & Record<string, any>;
export async function subscribe(handler: (e: UiEvent) => void): Promise<() => void> {
  const w = window as any;
  if (!w.__TAURI_INTERNALS__) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<UiEvent>("halokey", (ev) => handler(ev.payload));
  return un;
}
```

- [ ] **Step 4: 安装依赖并构建**

Run: `cd ~/Desktop/halokey/ui && npm install && npm run build && ls dist`
Expected: 输出含 `index.html`、`halo.html`、`assets/`。

- [ ] **Step 5: 接到 Tauri**

`apps/halokey/tauri.conf.json` 的 `build` 改为：

```json
"build": {
  "beforeDevCommand": "npm --prefix ../../ui run dev",
  "beforeBuildCommand": "npm --prefix ../../ui run build",
  "devUrl": "http://localhost:5173",
  "frontendDist": "../../ui/dist"
}
```

`app.security` 增加 `"devCsp": null`（开发时 Vite 需要 ws 与内联脚本），正式 CSP 不变。`app.windows[0]` 的 `width/height` 改为 1100×760，`minWidth/minHeight` 改为 960×640。

`.gitignore` 追加：

```
ui/node_modules/
ui/dist/
```

`.github/workflows/ci.yml` 在 "Format" 步骤前插入：

```yaml
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: ui/package-lock.json
      - name: Frontend
        working-directory: ui
        run: |
          npm ci
          npm run check
          npm test
          npm run build
```

- [ ] **Step 6: 验证 Tauri 能打包并启动**

Run: `cd ~/Desktop/halokey && source ~/.cargo/env && cargo build -p halokey-app && pkill -x halokey-app; (DISPLAY=:1 nohup ./target/debug/halokey-app > /tmp/halokey-app.log 2>&1 &); sleep 4; pgrep -x halokey-app`
Expected: 进程存活；托盘点开设置窗显示 "前端骨架已接通"；按旋钮光环仍能出现（圆泡版）。

- [ ] **Step 7: 提交**

```bash
git add -A && git commit -m "build(ui): Svelte 5 + Vite frontend skeleton wired into Tauri

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: 配置类型与标签

**Files:**
- Create: `ui/src/model/keymap.ts`, `ui/src/model/labels.ts`
- Test: `ui/src/model/__tests__/keymap.test.ts`, `ui/src/model/__tests__/labels.test.ts`

**Interfaces:**
- Produces:
  - `type KeyEntry = { type: number; code: number }`
  - `type HostAction = { keyboard: { _0: KeyEntry[] } } | { application: { _0: ApplicationTarget } } | { macro: { _0: MacroStep[] } } | { library: { _0: string } } | { momentaryLayer: { _0: number } } | { pointerMove: { _0: { dx: number; dy: number } } } | { halo: { _0: string } } | { [other: string]: unknown }`（最后一项让未知动作也能通过）
  - `type ControlActionMap`、`type HostLayer`、`type Halo`、`type HostKeymap`（字段名与 Rust 的 camelCase 一致）
  - `actionKind(a: HostAction): string`、`effectivePress(c: ControlActionMap): HostAction`、`effectiveDouble(c)`、`effectiveLong(c)`（后两者可返回 `null`）
  - `labels.ts`：`CONTROL_NAMES: readonly string[]`（6 项）、`keyName(e: KeyEntry): string`、`actionLabel(a: HostAction | null, entries?: KeyEntry[] | null, halos?: Halo[]): string`、`GESTURES = ["press","double","long"] as const`、`type Gesture`

- [ ] **Step 1: 写失败的测试**

`ui/src/model/__tests__/keymap.test.ts`：

```ts
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { effectivePress, effectiveDouble, actionKind, type HostKeymap } from "../keymap";

const example = (name: string): HostKeymap =>
  JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples", name), "utf8"));

describe("keymap types", () => {
  it("round-trips every example without losing fields", () => {
    for (const f of ["knob-moves-pointer.json", "switch-apps-and-macro.json", "halo-menu.json"]) {
      const raw = readFileSync(resolve(__dirname, "../../../../docs/examples", f), "utf8");
      const map: HostKeymap = JSON.parse(raw);
      expect(JSON.parse(JSON.stringify(map))).toEqual(JSON.parse(raw));
      expect(map.controls).toHaveLength(6);
    }
  });
  it("resolves effective actions and keeps unknown kinds", () => {
    const map = example("halo-menu.json");
    const knob = map.controls.find((c) => c.index === 3)!;
    expect(actionKind(effectivePress(knob))).toBe("halo");
    expect(effectiveDouble(knob)).toBeNull();
    const weird = { futureAction: { _0: 1 } } as any;
    expect(actionKind(weird)).toBe("futureAction");
  });
});
```

`ui/src/model/__tests__/labels.test.ts`：

```ts
import { describe, it, expect } from "vitest";
import { keyName, actionLabel, CONTROL_NAMES } from "../labels";

describe("labels", () => {
  it("names keys, mouse entries and actions", () => {
    expect(CONTROL_NAMES).toHaveLength(6);
    expect(keyName({ type: 2, code: 0x28 })).toBe("Enter");
    expect(keyName({ type: 2, code: 0x04 })).toBe("A");
    expect(keyName({ type: 2, code: 0x68 })).toBe("F13");
    expect(keyName({ type: 7, code: 6 })).toBe("滚轮↓");
    expect(keyName({ type: 9, code: 1 })).toBe("类型9 0x01");
    expect(actionLabel(null, [{ type: 2, code: 0xe3 }, { type: 2, code: 0x04 }])).toBe("⌘A");
    expect(actionLabel(null, [{ type: 2, code: 0 }])).toBe("无");
    expect(actionLabel({ pointerMove: { _0: { dx: 0, dy: 40 } } })).toBe("指针 ↓40");
    expect(actionLabel({ halo: { _0: "main" } }, null, [{ id: "main", name: "主菜单", options: [] }])).toBe("光环「主菜单」");
    expect(actionLabel({ futureAction: { _0: 1 } } as any)).toBe("复杂动作");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd ui && npx vitest run src/model`
Expected: FAIL，找不到模块 `../keymap` / `../labels`。

- [ ] **Step 3: 实现 keymap.ts**

```ts
// 与 Rust HostKeymap 一一对应的类型；前端只读写这个形状，不做语义校验（校验在 Rust）。
export type KeyEntry = { type: number; code: number };
export type ApplicationTarget = { bundleIdentifier: string; path: string; name: string };
export type MacroStep =
  | { keyboard: { _0: KeyEntry[] } }
  | { application: { _0: ApplicationTarget } }
  | { delay: { _0: number } };
export type PointerDelta = { dx: number; dy: number };
export type HostAction =
  | { keyboard: { _0: KeyEntry[] } }
  | { application: { _0: ApplicationTarget } }
  | { macro: { _0: MacroStep[] } }
  | { library: { _0: string } }
  | { momentaryLayer: { _0: number } }
  | { pointerMove: { _0: PointerDelta } }
  | { halo: { _0: string } }
  | { [other: string]: unknown };
export type Behavior = "hold" | "tap" | "burst";
export type Gesture = "press" | "double" | "long";
export const GESTURES: readonly Gesture[] = ["press", "double", "long"];

export type ControlActionMap = {
  index: number;
  press: KeyEntry[];
  doublePress?: KeyEntry[] | null;
  longPress?: KeyEntry[] | null;
  pressAction?: HostAction | null;
  doublePressAction?: HostAction | null;
  longPressAction?: HostAction | null;
  pressBehavior?: Behavior;
  pressTapCount?: number;
  doublePressBehavior?: Behavior;
  doublePressTapCount?: number;
  longPressBehavior?: Behavior;
  longPressTapCount?: number;
  inheritedGestures?: ("press" | "doublePress" | "longPress")[];
};
export type HostLayer = { id: number; controls: ControlActionMap[] };
export type NamedHostAction = { id: string; name: string; slot: number; action: HostAction };
export type HaloOption = { label: string; action: HostAction };
export type Halo = { id: string; name: string; options: HaloOption[] };
export type HostKeymap = {
  version: number;
  controls: ControlActionMap[];
  layers?: HostLayer[];
  actionLibrary?: NamedHostAction[];
  halos?: Halo[];
  doublePressWindow: number;
  longPressThreshold: number;
};

export function actionKind(a: HostAction): string {
  return Object.keys(a)[0] ?? "unknown";
}
export function actionValue<T = unknown>(a: HostAction): T {
  return (a as any)[actionKind(a)]?._0 as T;
}
export function effectivePress(c: ControlActionMap): HostAction {
  return c.pressAction ?? { keyboard: { _0: c.press } };
}
export function effectiveDouble(c: ControlActionMap): HostAction | null {
  return c.doublePressAction ?? (c.doublePress ? { keyboard: { _0: c.doublePress } } : null);
}
export function effectiveLong(c: ControlActionMap): HostAction | null {
  return c.longPressAction ?? (c.longPress ? { keyboard: { _0: c.longPress } } : null);
}
export const isRotation = (index: number) => index === 4 || index === 5;
```

- [ ] **Step 4: 实现 labels.ts**

```ts
import { actionKind, actionValue, type HostAction, type KeyEntry, type Halo, type ApplicationTarget, type PointerDelta } from "./keymap";

export const CONTROL_NAMES = ["顶部按键", "中间按键", "底部按键", "旋钮按下", "右旋", "左旋"] as const;
const SPECIAL: Record<number, string> = {
  0x00: "无", 0x01: "无效码", 0x28: "Enter", 0x29: "Esc", 0x2a: "⌫", 0x2b: "Tab", 0x2c: "Space", 0x2d: "−", 0x2e: "=", 0x2f: "[", 0x30: "]",
  0x31: "\\", 0x32: "Non-US #", 0x33: ";", 0x34: "'", 0x35: "`", 0x36: ",", 0x37: ".", 0x38: "/", 0x39: "Caps Lock",
  0x46: "Print Screen", 0x47: "Scroll Lock", 0x48: "Pause", 0x49: "Insert", 0x4a: "Home", 0x4b: "Page Up", 0x4c: "Delete", 0x4d: "End",
  0x4e: "Page Down", 0x4f: "→", 0x50: "←", 0x51: "↓", 0x52: "↑", 0x53: "Num Lock", 0x54: "KP /", 0x55: "KP *", 0x56: "KP −", 0x57: "KP +",
  0x58: "KP Enter", 0x62: "KP 0", 0x63: "KP .", 0x65: "Menu",
  0xe0: "左 Control", 0xe1: "左 Shift", 0xe2: "左 Alt", 0xe3: "左 Super", 0xe4: "右 Control", 0xe5: "右 Shift", 0xe6: "右 Alt", 0xe7: "右 Super",
};
const MOUSE = ["左键", "右键", "中键", "鼠标键 4", "鼠标键 5", "滚轮↑", "滚轮↓"];
const MOD_SYMBOL: Record<number, string> = { 0xe0: "⌃", 0xe1: "⇧", 0xe2: "⌥", 0xe3: "⌘" };

export function keyName(e: KeyEntry): string {
  if (e.type === 7) return MOUSE[e.code] ?? `鼠标 0x${e.code.toString(16).padStart(2, "0")}`;
  if (e.type !== 2) return `类型${e.type} 0x${e.code.toString(16).padStart(2, "0")}`;
  const c = e.code;
  if (SPECIAL[c]) return SPECIAL[c];
  if (c >= 0x04 && c <= 0x1d) return String.fromCharCode(65 + c - 4);
  if (c >= 0x1e && c <= 0x27) return String((c - 0x1d) % 10);
  if (c >= 0x3a && c <= 0x45) return `F${c - 0x39}`;
  if (c >= 0x59 && c <= 0x61) return `KP ${c - 0x58}`;
  if (c >= 0x68 && c <= 0x73) return `F${c - 0x68 + 13}`;
  return `0x${c.toString(16).padStart(2, "0")}`;
}

export function entriesLabel(entries: KeyEntry[]): string {
  const active = entries.filter((e) => !(e.type === 2 && e.code === 0));
  if (!active.length) return "无";
  const mods = active.filter((e) => e.type === 2 && MOD_SYMBOL[e.code]).map((e) => MOD_SYMBOL[e.code]).join("");
  const rest = active.filter((e) => !(e.type === 2 && MOD_SYMBOL[e.code])).map(keyName).join(" + ");
  return mods && rest ? `${mods}${rest}` : mods || rest;
}

export function actionLabel(a: HostAction | null, entries?: KeyEntry[] | null, halos?: Halo[]): string {
  if (!a) return entries ? entriesLabel(entries) : "无";
  switch (actionKind(a)) {
    case "keyboard": return entriesLabel(actionValue<KeyEntry[]>(a));
    case "application": return `切换到 ${actionValue<ApplicationTarget>(a).name}`;
    case "macro": return `宏（${(actionValue<unknown[]>(a)).length} 步）`;
    case "library": return `动作库 ${String(actionValue<string>(a)).slice(0, 8)}`;
    case "momentaryLayer": return `MO(${actionValue<number>(a)})`;
    case "pointerMove": {
      const { dx, dy } = actionValue<PointerDelta>(a);
      const parts = [dx ? `${dx > 0 ? "→" : "←"}${Math.abs(dx)}` : "", dy ? `${dy > 0 ? "↓" : "↑"}${Math.abs(dy)}` : ""].filter(Boolean);
      return `指针 ${parts.join(" ")}`;
    }
    case "halo": { const id = actionValue<string>(a); return `光环「${halos?.find((h) => h.id === id)?.name ?? id}」`; }
    default: return "复杂动作";
  }
}
```

- [ ] **Step 5: 运行测试**

Run: `cd ui && npx vitest run src/model`
Expected: PASS（2 个文件）。

- [ ] **Step 6: 提交**

```bash
git add ui/src/model && git commit -m "feat(ui): keymap types and labels with example round-trip tests

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: 键码面板的分类与键帽表

**Files:**
- Create: `ui/src/model/codes.ts`
- Test: `ui/src/model/__tests__/codes.test.ts`

**Interfaces:**
- Produces: `type Keycap = { label: string; entry: KeyEntry }`、`type Category = { id: CategoryId; name: string; keycaps?: Keycap[] }`、`CATEGORIES: Category[]`（顺序：常用 字母 功能键 数字键盘 鼠标 | 组合键 Layer APP 宏 光环 指针）、`CategoryId = "common"|"letters"|"fkeys"|"keypad"|"mouse"|"combo"|"layer"|"app"|"macro"|"halo"|"pointer"`、`isAllowedKeyboardCode(code: number): boolean`（与 Rust `is_allowed_keyboard_code` 一致）。

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect } from "vitest";
import { CATEGORIES, isAllowedKeyboardCode } from "../codes";

describe("codes", () => {
  it("lists categories in the agreed order", () => {
    expect(CATEGORIES.map((c) => c.id)).toEqual(["common", "letters", "fkeys", "keypad", "mouse", "combo", "layer", "app", "macro", "halo", "pointer"]);
  });
  it("keycaps use only allowed codes and have no duplicates", () => {
    const seen = new Set<string>();
    for (const c of CATEGORIES) for (const k of c.keycaps ?? []) {
      const key = `${k.entry.type}:${k.entry.code}`;
      expect(seen.has(key), key).toBe(false);
      seen.add(key);
      if (k.entry.type === 2) expect(isAllowedKeyboardCode(k.entry.code), key).toBe(true);
      else expect(k.entry.type).toBe(7);
    }
    expect(seen.size).toBeGreaterThan(90);
  });
  it("mirrors the Rust allowed-code predicate", () => {
    expect(isAllowedKeyboardCode(0x00)).toBe(true);
    expect(isAllowedKeyboardCode(0x02)).toBe(false);
    expect(isAllowedKeyboardCode(0x64)).toBe(false);
    expect(isAllowedKeyboardCode(0x65)).toBe(true);
    expect(isAllowedKeyboardCode(0x73)).toBe(true);
    expect(isAllowedKeyboardCode(0x74)).toBe(false);
    expect(isAllowedKeyboardCode(0xe7)).toBe(true);
  });
});
```

- [ ] **Step 2: 运行确认失败** — `cd ui && npx vitest run src/model/__tests__/codes.test.ts` → FAIL。

- [ ] **Step 3: 实现 codes.ts**

```ts
import type { KeyEntry } from "./keymap";
import { keyName } from "./labels";

export type Keycap = { label: string; entry: KeyEntry };
export type CategoryId = "common" | "letters" | "fkeys" | "keypad" | "mouse" | "combo" | "layer" | "app" | "macro" | "halo" | "pointer";
export type Category = { id: CategoryId; name: string; keycaps?: Keycap[] };

// 与 crates/proto keyconfig::is_allowed_keyboard_code 保持一致。
export function isAllowedKeyboardCode(code: number): boolean {
  return code === 0 || code === 1 || (code >= 0x04 && code <= 0x63) || code === 0x65 || (code >= 0x68 && code <= 0x73) || (code >= 0xe0 && code <= 0xe7);
}
const kb = (code: number, label?: string): Keycap => ({ label: label ?? keyName({ type: 2, code }), entry: { type: 2, code } });
const range = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => kb(from + i));

export const CATEGORIES: Category[] = [
  { id: "common", name: "常用", keycaps: [kb(0x28), kb(0x29), kb(0x2a), kb(0x2b), kb(0x2c), kb(0x39), kb(0x65), kb(0x46), kb(0x47), kb(0x48), kb(0x49), kb(0x4a), kb(0x4b), kb(0x4c), kb(0x4d), kb(0x4e), kb(0x4f), kb(0x50), kb(0x51), kb(0x52), ...range(0xe0, 0xe7), kb(0x01)] },
  { id: "letters", name: "字母", keycaps: [...range(0x04, 0x1d), ...range(0x1e, 0x27), ...range(0x2d, 0x38)] },
  { id: "fkeys", name: "功能键", keycaps: [...range(0x3a, 0x45), ...range(0x68, 0x73)] },
  { id: "keypad", name: "数字键盘", keycaps: range(0x53, 0x63) },
  { id: "mouse", name: "鼠标", keycaps: Array.from({ length: 7 }, (_, code) => ({ label: keyName({ type: 7, code }), entry: { type: 7, code } })) },
  { id: "combo", name: "组合键" },
  { id: "layer", name: "Layer" },
  { id: "app", name: "APP" },
  { id: "macro", name: "宏" },
  { id: "halo", name: "光环" },
  { id: "pointer", name: "指针" },
];
```

- [ ] **Step 4: 运行测试** → PASS。
- [ ] **Step 5: 提交** `git add ui/src/model && git commit -m "feat(ui): palette categories and keycap tables"`（带 Co-Authored-By）。

---

### Task 4: 键位草稿 reducer

**Files:**
- Create: `ui/src/model/draft.ts`
- Test: `ui/src/model/__tests__/draft.test.ts`

**Interfaces:**
- Produces:
  - `type Selection = { layer: number; index: number; gesture: Gesture }`
  - `type Draft = { saved: HostKeymap | null; map: HostKeymap | null; selection: Selection; dirty: boolean }`
  - `initialDraft(): Draft`
  - `loadSaved(d, map): Draft`（dirty 时不替换 `map`，只更新 `saved`）
  - `select(d, sel): Draft`
  - `assignEntries(d, entries: KeyEntry[]): Draft`（写到选中手势；非基础层缺记录时创建覆盖记录）
  - `assignAction(d, action: HostAction): Draft`
  - `clearGesture(d): Draft`（单击 = `[{type:2,code:0}]`，双击 / 长按 = 删除）
  - `setTransparent(d): Draft`（非基础层：把该手势加入 `inheritedGestures`；基础层无操作）
  - `setBehavior(d, b: Behavior): Draft`、`setTapCount(d, n: number): Draft`
  - `revert(d): Draft`（回到 saved）、`markSaved(d): Draft`
  - `controlAt(map, layer, index): ControlActionMap | undefined`、`resolvedControl(map, layer, index): ControlActionMap`（基础层 + 该层覆盖的叠加，供卡片显示）

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { initialDraft, loadSaved, select, assignEntries, assignAction, clearGesture, setTransparent, setBehavior, setTapCount, revert, markSaved, controlAt, resolvedControl } from "../draft";
import type { HostKeymap } from "../keymap";

const load = (): HostKeymap => JSON.parse(readFileSync(resolve(__dirname, "../../../../docs/examples/halo-menu.json"), "utf8"));

describe("draft reducer", () => {
  it("loads, selects, assigns and tracks dirty", () => {
    let d = loadSaved(initialDraft(), load());
    expect(d.dirty).toBe(false);
    d = select(d, { layer: 0, index: 1, gesture: "press" });
    d = assignEntries(d, [{ type: 2, code: 0x2b }]);
    expect(d.dirty).toBe(true);
    expect(controlAt(d.map!, 0, 1)!.press).toEqual([{ type: 2, code: 0x2b }]);
    expect(controlAt(d.map!, 0, 1)!.pressAction).toBeUndefined();
    d = assignAction(d, { pointerMove: { _0: { dx: 20, dy: 0 } } });
    expect(controlAt(d.map!, 0, 1)!.pressAction).toEqual({ pointerMove: { _0: { dx: 20, dy: 0 } } });
    d = revert(d);
    expect(d.dirty).toBe(false);
    expect(controlAt(d.map!, 0, 1)!.press).toEqual(load().controls[1].press);
  });
  it("does not overwrite a dirty draft when a new saved map arrives", () => {
    let d = loadSaved(initialDraft(), load());
    d = assignEntries(select(d, { layer: 0, index: 0, gesture: "press" }), [{ type: 2, code: 0x04 }]);
    const incoming = load(); incoming.doublePressWindow = 0.3;
    d = loadSaved(d, incoming);
    expect(d.map!.doublePressWindow).toBe(0.25);
    expect(d.saved!.doublePressWindow).toBe(0.3);
    d = markSaved(d);
    expect(d.dirty).toBe(false);
  });
  it("creates layer overrides on demand and supports transparency", () => {
    let d = loadSaved(initialDraft(), load());
    d = select(d, { layer: 2, index: 1, gesture: "long" });
    d = assignEntries(d, [{ type: 2, code: 0x29 }]);
    const layer = d.map!.layers!.find((l) => l.id === 2)!;
    expect(layer.controls).toHaveLength(1);
    expect(layer.controls[0]!.longPress).toEqual([{ type: 2, code: 0x29 }]);
    expect(layer.controls[0]!.inheritedGestures).toEqual(["press", "doublePress"]);
    expect(resolvedControl(d.map!, 2, 1).press).toEqual(load().controls[1].press);
    d = setTransparent(select(d, { layer: 2, index: 1, gesture: "long" }));
    expect(controlAt(d.map!, 2, 1)!.inheritedGestures).toEqual(["press", "doublePress", "longPress"]);
    expect(controlAt(d.map!, 2, 1)!.longPress).toBeUndefined();
  });
  it("clears gestures, rejects rotation extras and clamps tap counts", () => {
    let d = loadSaved(initialDraft(), load());
    d = clearGesture(select(d, { layer: 0, index: 0, gesture: "press" }));
    expect(controlAt(d.map!, 0, 0)!.press).toEqual([{ type: 2, code: 0 }]);
    d = clearGesture(select(d, { layer: 0, index: 0, gesture: "double" }));
    expect(controlAt(d.map!, 0, 0)!.doublePress).toBeUndefined();
    const before = JSON.stringify(d.map);
    d = assignEntries(select(d, { layer: 0, index: 4, gesture: "long" }), [{ type: 2, code: 0x04 }]);
    expect(JSON.stringify(d.map)).toBe(before);
    d = setBehavior(select(d, { layer: 0, index: 4, gesture: "press" }), "burst");
    expect(controlAt(d.map!, 0, 4)!.pressBehavior).toBe("burst");
    const beforeHold = JSON.stringify(d.map);
    d = setBehavior(select(d, { layer: 0, index: 4, gesture: "press" }), "hold");
    d = setBehavior(select(d, { layer: 0, index: 0, gesture: "double" }), "hold");
    expect(JSON.stringify(d.map)).toBe(beforeHold);
    d = setTapCount(select(d, { layer: 0, index: 0, gesture: "press" }), 99);
    expect(controlAt(d.map!, 0, 0)!.pressTapCount).toBe(20);
  });
});
```

- [ ] **Step 2: 运行确认失败** → FAIL。

- [ ] **Step 3: 实现 draft.ts**

```ts
import type { Behavior, ControlActionMap, Gesture, HostAction, HostKeymap, KeyEntry } from "./keymap";
import { isRotation } from "./keymap";

export type Selection = { layer: number; index: number; gesture: Gesture };
export type Draft = { saved: HostKeymap | null; map: HostKeymap | null; selection: Selection; dirty: boolean };

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));
const GKEY = { press: "press", double: "doublePress", long: "longPress" } as const;
const AKEY = { press: "pressAction", double: "doublePressAction", long: "longPressAction" } as const;
const BKEY = { press: "pressBehavior", double: "doublePressBehavior", long: "longPressBehavior" } as const;
const CKEY = { press: "pressTapCount", double: "doublePressTapCount", long: "longPressTapCount" } as const;

export const initialDraft = (): Draft => ({ saved: null, map: null, selection: { layer: 0, index: 0, gesture: "press" }, dirty: false });

export function loadSaved(d: Draft, map: HostKeymap): Draft {
  return { ...d, saved: clone(map), map: d.dirty && d.map ? d.map : clone(map) };
}
export const select = (d: Draft, selection: Selection): Draft => ({ ...d, selection });
export const revert = (d: Draft): Draft => ({ ...d, map: d.saved ? clone(d.saved) : null, dirty: false });
export const markSaved = (d: Draft): Draft => ({ ...d, saved: d.map ? clone(d.map) : null, dirty: false });

export function controlAt(map: HostKeymap, layer: number, index: number): ControlActionMap | undefined {
  if (layer === 0) return map.controls.find((c) => c.index === index);
  return map.layers?.find((l) => l.id === layer)?.controls.find((c) => c.index === index);
}

/** 基础层加该层覆盖的叠加结果，只用于显示。 */
export function resolvedControl(map: HostKeymap, layer: number, index: number): ControlActionMap {
  const base = clone(map.controls.find((c) => c.index === index)!);
  const over = layer === 0 ? undefined : controlAt(map, layer, index);
  if (!over) return base;
  const inh = new Set(over.inheritedGestures ?? []);
  if (!inh.has("press")) Object.assign(base, { press: over.press, pressAction: over.pressAction, pressBehavior: over.pressBehavior, pressTapCount: over.pressTapCount });
  if (!inh.has("doublePress")) Object.assign(base, { doublePress: over.doublePress, doublePressAction: over.doublePressAction, doublePressBehavior: over.doublePressBehavior, doublePressTapCount: over.doublePressTapCount });
  if (!inh.has("longPress")) Object.assign(base, { longPress: over.longPress, longPressAction: over.longPressAction, longPressBehavior: over.longPressBehavior, longPressTapCount: over.longPressTapCount });
  return base;
}

/** 取得可写的控件记录；非基础层缺记录时创建一条全透明的覆盖记录。 */
function writable(map: HostKeymap, layer: number, index: number): ControlActionMap {
  if (layer === 0) return map.controls.find((c) => c.index === index)!;
  map.layers ??= [];
  let l = map.layers.find((x) => x.id === layer);
  if (!l) { l = { id: layer, controls: [] }; map.layers.push(l); map.layers.sort((a, b) => a.id - b.id); }
  let c = l.controls.find((x) => x.index === index);
  if (!c) { c = { index, press: [{ type: 2, code: 0 }], inheritedGestures: ["press", "doublePress", "longPress"] }; l.controls.push(c); }
  return c;
}

function mutate(d: Draft, f: (map: HostKeymap, c: ControlActionMap, g: Gesture) => boolean): Draft {
  if (!d.map) return d;
  const { layer, index, gesture } = d.selection;
  if (isRotation(index) && gesture !== "press") return d;
  const map = clone(d.map);
  const c = writable(map, layer, index);
  if (!f(map, c, gesture)) return d;
  return { ...d, map, dirty: true };
}
const uninherit = (c: ControlActionMap, g: Gesture) => { c.inheritedGestures = (c.inheritedGestures ?? []).filter((x) => x !== GKEY[g]); if (!c.inheritedGestures.length) delete c.inheritedGestures; };

export const assignEntries = (d: Draft, entries: KeyEntry[]) => mutate(d, (_m, c, g) => {
  uninherit(c, g); (c as any)[GKEY[g]] = entries; delete (c as any)[AKEY[g]]; return true;
});
export const assignAction = (d: Draft, action: HostAction) => mutate(d, (_m, c, g) => {
  uninherit(c, g); (c as any)[AKEY[g]] = action; if (g !== "press") delete (c as any)[GKEY[g]]; return true;
});
export const clearGesture = (d: Draft) => mutate(d, (_m, c, g) => {
  uninherit(c, g); delete (c as any)[AKEY[g]];
  if (g === "press") c.press = [{ type: 2, code: 0 }]; else delete (c as any)[GKEY[g]];
  return true;
});
export const setTransparent = (d: Draft) => mutate(d, (_m, c, g) => {
  if (d.selection.layer === 0) return false;
  delete (c as any)[AKEY[g]]; if (g !== "press") delete (c as any)[GKEY[g]];
  const set = new Set(c.inheritedGestures ?? []); set.add(GKEY[g]);
  c.inheritedGestures = (["press", "doublePress", "longPress"] as const).filter((k) => set.has(k));
  return true;
});
export const setBehavior = (d: Draft, b: Behavior) => mutate(d, (_m, c, g) => {
  if (isRotation(d.selection.index) && b === "hold") return false;
  if (g === "double" && b === "hold") return false;
  (c as any)[BKEY[g]] = b; return true;
});
export const setTapCount = (d: Draft, n: number) => mutate(d, (_m, c, g) => {
  (c as any)[CKEY[g]] = Math.min(20, Math.max(2, Math.round(n))); return true;
});
```

注意 `setBehavior` 对"旋转 + hold"和"双击 + hold"都返回 false，草稿原样不动（测试用 JSON 快照确认）。

- [ ] **Step 4: 运行测试** → PASS（4 个用例）。
- [ ] **Step 5: 提交** `git add ui/src/model && git commit -m "feat(ui): keymap draft reducer with layer overrides and transparency"`。

---

### Task 5: Tauri 桥与浏览器 mock

**Files:**
- Modify: `ui/src/lib/tauri.ts`（替换 Task 1 的最小版）
- Test: `ui/src/lib/__tests__/tauri.test.ts`

**Interfaces:**
- Produces:
  - `type UiSnapshot = { connected: boolean; online: boolean | null; battery: [number | null, number, boolean | null] | null; heartbeats: number; echoes: number; injected: number; lastError: string | null; configPath: string; paused: boolean; standby: boolean | null; link: boolean | null; activeLayers: number[] }`
  - `type UiEvent = {kind:"connected"}|{kind:"disconnected"}|{kind:"key";control:number;name:string;pressed:boolean;at:number}|{kind:"battery";percent:number|null;millivolts:number;charging:boolean|null}|{kind:"haloShow";halo:Halo;selected:number}|{kind:"haloSelect";selected:number}|{kind:"haloHide"}|{kind:"error";text:string}|{kind:"paused"}|{kind:"resumed"}|{kind:"stopped"}`
  - `api.snapshot(): Promise<UiSnapshot>`、`api.getKeymap(): Promise<HostKeymap>`、`api.validateKeymap(map): Promise<void>`（失败 reject 中文错误）、`api.saveKeymap(map): Promise<void>`、`api.deviceInfo()`, `api.powerInfo()`, `api.getSettings()`, `api.setIdleMinutes(n)`, `api.getAutostart()`, `api.setAutostart(b)`
  - `subscribe(handler): Promise<() => void>`、`isTauri(): boolean`
  - mock：无 Tauri 时 `api.*` 返回内存数据（`docs/examples/halo-menu.json` 内容以 import 方式打进来），`validateKeymap` 做最基本检查（6 个控件），`subscribe` 每 3 秒推一条假 key 事件。

- [ ] **Step 1: 写失败的测试**

```ts
import { describe, it, expect } from "vitest";
import { api, isTauri } from "../tauri";

describe("tauri bridge (mock mode in jsdom)", () => {
  it("reports no tauri and serves a keymap", async () => {
    expect(isTauri()).toBe(false);
    const map = await api.getKeymap();
    expect(map.controls).toHaveLength(6);
    const snap = await api.snapshot();
    expect(snap.connected).toBe(true);
    expect(Array.isArray(snap.activeLayers)).toBe(true);
  });
  it("validates and saves in memory", async () => {
    const map = await api.getKeymap();
    await expect(api.validateKeymap({ ...map, controls: map.controls.slice(0, 5) })).rejects.toMatch(/六个控件/);
    map.doublePressWindow = 0.3;
    await api.saveKeymap(map);
    expect((await api.getKeymap()).doublePressWindow).toBe(0.3);
  });
});
```

- [ ] **Step 2: 运行确认失败** → FAIL。

- [ ] **Step 3: 实现 tauri.ts**

```ts
import type { HostKeymap, Halo } from "../model/keymap";
import example from "../../../docs/examples/halo-menu.json";

export type UiSnapshot = { connected: boolean; online: boolean | null; battery: [number | null, number, boolean | null] | null; heartbeats: number; echoes: number; injected: number; lastError: string | null; configPath: string; paused: boolean; standby: boolean | null; link: boolean | null; activeLayers: number[] };
export type UiEvent =
  | { kind: "connected" } | { kind: "disconnected" }
  | { kind: "key"; control: number; name: string; pressed: boolean; at: number }
  | { kind: "battery"; percent: number | null; millivolts: number; charging: boolean | null }
  | { kind: "haloShow"; halo: Halo; selected: number } | { kind: "haloSelect"; selected: number } | { kind: "haloHide" }
  | { kind: "error"; text: string } | { kind: "paused" } | { kind: "resumed" } | { kind: "stopped" };
export type UiLight = { kind: number; workTime: number; breatheLevel: number; breatheBrightness: number; alwaysOnBrightness: number };
export type UiLighting = { mode: number; brightness: number; lights: UiLight[] };
export type UiDeviceInfo = { firmware: string | null; dongleFirmware: string | null; noiseReduction: number[] | null; lighting: UiLighting | null; errors: string[] };
export type UiPower = { standby: boolean | null; standbyDelay: number | null; sleepDelay: number | null; micEnabled: boolean | null; noiseReduction: number | null; errors: string[] };
export type AppSettings = { idleMinutes: number };

export const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// 浏览器里（npm run dev / vitest）没有 Tauri，用内存数据让界面能跑。
let memory: HostKeymap = JSON.parse(JSON.stringify(example));
const mockSnapshot = (): UiSnapshot => ({ connected: true, online: true, battery: [60, 3791, false], heartbeats: 1024, echoes: 1024, injected: 58, lastError: null, configPath: "~/.config/halokey/keymap.json", paused: false, standby: false, link: true, activeLayers: [] });
function mockValidate(map: HostKeymap): void {
  const idx = new Set(map.controls.map((c) => c.index));
  if (map.controls.length !== 6 || idx.size !== 6) throw "配置必须包含且仅包含六个控件，每个控件只能出现一次。";
}

export const api = {
  snapshot: (): Promise<UiSnapshot> => isTauri() ? invoke("snapshot") : Promise.resolve(mockSnapshot()),
  getKeymap: (): Promise<HostKeymap> => isTauri() ? invoke("get_keymap") : Promise.resolve(JSON.parse(JSON.stringify(memory))),
  validateKeymap: async (keymap: HostKeymap): Promise<void> => { if (isTauri()) return invoke("validate_keymap", { keymap }); mockValidate(keymap); },
  saveKeymap: async (keymap: HostKeymap): Promise<void> => { if (isTauri()) return invoke("save_keymap", { keymap }); mockValidate(keymap); memory = JSON.parse(JSON.stringify(keymap)); },
  deviceInfo: (): Promise<UiDeviceInfo> => isTauri() ? invoke("device_info") : Promise.resolve({ firmware: "4.4.2", dongleFirmware: "4.4.2", noiseReduction: [1, 64, 6, 232], lighting: { mode: 2, brightness: 14, lights: [] }, errors: [] }),
  powerInfo: (): Promise<UiPower> => isTauri() ? invoke("power_info") : Promise.resolve({ standby: false, standbyDelay: 300, sleepDelay: 3600, micEnabled: true, noiseReduction: 1, errors: [] }),
  getSettings: (): Promise<AppSettings> => isTauri() ? invoke("get_settings") : Promise.resolve({ idleMinutes: 5 }),
  setIdleMinutes: (minutes: number): Promise<void> => isTauri() ? invoke("set_idle_minutes", { minutes }) : Promise.resolve(),
  getAutostart: (): Promise<boolean> => isTauri() ? invoke("get_autostart") : Promise.resolve(false),
  setAutostart: (enabled: boolean): Promise<boolean> => isTauri() ? invoke("set_autostart", { enabled }) : Promise.resolve(enabled),
};

export async function subscribe(handler: (e: UiEvent) => void): Promise<() => void> {
  if (!isTauri()) {
    const timer = setInterval(() => handler({ kind: "key", control: 4, name: "右旋", pressed: true, at: performance.now() / 1000 }), 3000);
    return () => clearInterval(timer);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<UiEvent>("halokey", (ev) => handler(ev.payload));
}
```

- [ ] **Step 4: 运行测试** → PASS。`npm run check` 也要过（`resolveJsonModule` 已开）。
- [ ] **Step 5: 提交** `git add ui/src/lib && git commit -m "feat(ui): typed Tauri bridge with an in-browser mock"`。

---

### Task 6: 主题、顶栏、页面切换与 i18n

**Files:**
- Create: `ui/src/lib/i18n.ts`, `ui/src/components/TopBar.svelte`, `ui/src/pages/Keymap.svelte`（占位）, `ui/src/pages/Settings.svelte`（占位）
- Modify: `ui/src/App.svelte`
- Test: `ui/src/lib/__tests__/i18n.test.ts`

**Interfaces:**
- Produces: `t(zh: string): string`、`lang(): "zh"|"en"`、`setLang(l)`（存 localStorage `halokey.lang`，触发 `window` 事件 `halokey-lang`）、`EN: Record<string,string>`；`TopBar` props：`{ page: "keymap"|"settings"; snapshot: UiSnapshot | null; dirty: boolean; onNavigate: (p) => void }`。

- [ ] **Step 1: 写失败的测试（i18n 完整性）**

```ts
import { describe, it, expect } from "vitest";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { EN, t, setLang } from "../i18n";

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
});
```

- [ ] **Step 2: 运行确认失败** → FAIL。

- [ ] **Step 3: 实现 i18n.ts（表从 `ui/legacy/i18n.js` 搬过来并补新词）**

```ts
export const EN: Record<string, string> = {
  "键位": "Keymap", "设置": "Settings", "已连接": "Connected", "未连接": "Disconnected", "空闲中，已交还控制": "Idle, control handed back",
  "接收器在，本体关机": "Receiver present, key unit asleep", "电量": "Battery", "固件": "Firmware", "未保存": "Unsaved",
  "顶部按键": "Top key", "中间按键": "Middle key", "底部按键": "Bottom key", "旋钮按下": "Knob press", "右旋": "Turn right", "左旋": "Turn left",
  "单击": "Press", "双击": "Double", "长按": "Long press", "保持按住": "Hold", "短按一次": "Tap once", "连按": "Burst", "次": "×",
  "常用": "Common", "字母": "Letters", "功能键": "F keys", "数字键盘": "Keypad", "鼠标": "Mouse", "组合键": "Combo", "APP": "APP", "宏": "Macro", "光环": "Halo", "指针": "Pointer",
  "撤销更改": "Discard changes", "保存到本机": "Save locally", "已保存": "Saved", "透明继承": "Transparent", "不执行动作": "No action", "关闭手势": "Disable gesture",
  "本层背光": "Layer backlight", "编辑…": "Edit…", "跟随 Layer 0": "Follow Layer 0",
  "Vibe Key": "Vibe Key", "灯光": "Lighting", "麦克风": "Microphone", "电源": "Power", "光环转盘": "Halo wheel", "键位配置": "Keymap file", "通用": "General",
  "状态": "Status", "本体在线": "Key unit online", "本体离线": "Key unit offline", "空闲后交还控制": "Hand back when idle", "永不": "Never", "分钟": "min",
  "登录时启动": "Launch at login", "界面语言": "Language", "刷新状态": "Refresh", "灯光模式": "Light mode", "全灭": "Off", "全亮": "All on", "工作": "Work",
  "全亮亮度": "All-on brightness", "休眠 / 自动关机": "Sleep / auto power-off", "待机延时": "Standby delay", "收音": "Capture", "硬件降噪": "Noise reduction",
  "各灯效果在键位页按 Layer 设置": "Per-light effects are set per layer on the Keymap page", "当前文件": "Current file", "关于": "About",
  "第 1 期：设置页只读；写入项在后续计划中": "Phase 1: settings are read-only; writes come in later phases",
  "点击上方的手势，再从下面选一个键码": "Pick a gesture above, then a keycode below", "指针步长": "Pointer step", "像素": "px",
  "在本机构建里还没有的功能": "Not available in this build yet", "复杂动作": "Complex action", "无": "None",
  "宏（": "Macro (", "切换到": "Switch to", "动作库": "Library", "心跳": "heartbeats", "回显": "echoes",
};
let current: "zh" | "en" = "zh";
try { current = (localStorage.getItem("halokey.lang") as "zh" | "en") || (navigator.language.startsWith("zh") ? "zh" : "en"); } catch { /* jsdom 或隐私模式 */ }
export const lang = () => current;
export const t = (zh: string) => (current === "en" ? EN[zh] ?? zh : zh);
export function setLang(l: "zh" | "en") {
  current = l;
  try { localStorage.setItem("halokey.lang", l); } catch { /* 忽略 */ }
  if (typeof window !== "undefined") window.dispatchEvent(new Event("halokey-lang"));
}
```

- [ ] **Step 4: TopBar、占位页与 App**

`ui/src/components/TopBar.svelte`：

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  let { page, snapshot, dirty, onNavigate }: { page: "keymap" | "settings"; snapshot: UiSnapshot | null; dirty: boolean; onNavigate: (p: "keymap" | "settings") => void } = $props();
  const status = $derived(!snapshot ? t("未连接") : !snapshot.connected ? t("未连接") : snapshot.paused ? t("空闲中，已交还控制") : snapshot.online === false ? t("接收器在，本体关机") : t("已连接"));
  const ok = $derived(!!snapshot?.connected && snapshot.online !== false && !snapshot.paused);
</script>

<header>
  <div class="logo">☰ HALOKEY</div>
  <nav>
    <button class:on={page === "keymap"} onclick={() => onNavigate("keymap")}>⌨ {t("键位")}{#if dirty}<i class="dot" title={t("未保存")}></i>{/if}</button>
    <button class:on={page === "settings"} onclick={() => onNavigate("settings")}>⚙ {t("设置")}</button>
  </nav>
  <div class="status">
    <div class:ok>● {status}</div>
    {#if snapshot?.battery}<div class="muted">🔋 {snapshot.battery[0] ?? "?"}% · {snapshot.battery[1]} mV</div>{/if}
  </div>
</header>

<style>
  header { display: flex; align-items: center; padding: 12px 22px; border-bottom: 1px solid var(--line); }
  .logo { font-weight: 700; letter-spacing: .12em; }
  nav { margin: 0 auto; display: flex; gap: 24px; }
  nav button { background: none; padding: 6px 2px; color: var(--muted); border-radius: 0; border-bottom: 2px solid transparent; position: relative; }
  nav button.on { color: var(--text); border-bottom-color: var(--accent); }
  .dot { position: absolute; top: 4px; right: -8px; width: 7px; height: 7px; border-radius: 50%; background: var(--accent); }
  .status { text-align: right; font-size: 13px; color: var(--muted); line-height: 1.3; }
  .status .ok { color: var(--ok); }
</style>
```

`ui/src/pages/Keymap.svelte` 与 `Settings.svelte` 先各放一行 `<p>{t("在本机构建里还没有的功能")}</p>`（Task 7 / Task 10 替换）。

`ui/src/App.svelte`：

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "./components/TopBar.svelte";
  import Keymap from "./pages/Keymap.svelte";
  import Settings from "./pages/Settings.svelte";
  import { api, subscribe, type UiSnapshot } from "./lib/tauri";
  import { initialDraft, loadSaved, type Draft } from "./model/draft";

  let page = $state<"keymap" | "settings">("keymap");
  let snapshot = $state<UiSnapshot | null>(null);
  let draft = $state<Draft>(initialDraft());
  let langTick = $state(0);

  async function refresh() {
    try { snapshot = await api.snapshot(); } catch (e) { console.error(e); }
    if (!draft.map && snapshot?.connected) try { draft = loadSaved(draft, await api.getKeymap()); } catch (e) { console.error(e); }
  }
  onMount(() => {
    refresh();
    const timer = setInterval(refresh, 2000);
    const relang = () => { langTick++; };
    window.addEventListener("halokey-lang", relang);
    let un: (() => void) | undefined;
    subscribe((e) => { if (e.kind === "connected") api.getKeymap().then((m) => { draft = loadSaved(draft, m); }); }).then((u) => { un = u; });
    return () => { clearInterval(timer); window.removeEventListener("halokey-lang", relang); un?.(); };
  });
</script>

{#key langTick}
  <TopBar {page} {snapshot} dirty={draft.dirty} onNavigate={(p) => (page = p)} />
  {#if page === "keymap"}
    <Keymap bind:draft {snapshot} />
  {:else}
    <Settings {snapshot} />
  {/if}
{/key}
```

- [ ] **Step 5: 运行** `npm test && npm run check && npm run build`，再 `cargo build -p halokey-app` 并启动，确认顶栏显示连接状态、两个页签可切换。
- [ ] **Step 6: 提交** `git add ui && git commit -m "feat(ui): top bar, page switching and i18n table"`。

---

### Task 7: 键位页舞台：Layer 条、设备渲染图、手势卡片

**Files:**
- Create: `ui/src/components/LayerBar.svelte`, `ui/src/components/DeviceView.svelte`, `ui/src/components/GestureCard.svelte`, `ui/src/components/KnobLabel.svelte`
- Modify: `ui/src/pages/Keymap.svelte`

**Interfaces:**
- `LayerBar` props：`{ layer: number; onSelect: (l: number) => void }`；右侧留一个 `<slot name="right">`（第 2 期放背光条）。
- `DeviceView` props：`{ lights: ("off"|"solid"|"breathe")[]; selectedIndex: number | null; onPick: (index: number) => void }`；四盏灯顺序：键 1、键 2、键 3、旋钮。
- `GestureCard` props：`{ index: number; control: ControlActionMap; selected: Gesture | null; halos: Halo[]; onPick: (g: Gesture) => void; layer: number; override?: ControlActionMap }`；非基础层且 `override` 缺少该手势或标记继承时显示 ▽。
- `KnobLabel` props：`{ index: 4 | 5; label: string; selected: boolean; onPick: () => void }`。
- `Keymap.svelte` props：`{ draft: Draft (bindable); snapshot: UiSnapshot | null }`。

- [ ] **Step 1: DeviceView.svelte（自绘 SVG）**

```svelte
<script lang="ts">
  let { lights, selectedIndex, onPick }: { lights: ("off" | "solid" | "breathe")[]; selectedIndex: number | null; onPick: (index: number) => void } = $props();
  const ring = (i: number) => lights[i] ?? "off";
</script>

<svg viewBox="0 0 120 300" width="118" height="295" class="device" role="img" aria-label="Vibe Key">
  <defs>
    <linearGradient id="body" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#f1efea"/><stop offset="1" stop-color="#d9d6d0"/></linearGradient>
    <radialGradient id="cap" cx=".4" cy=".35"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#cfccc6"/></radialGradient>
  </defs>
  <rect x="6" y="4" width="108" height="292" rx="24" fill="url(#body)" stroke="#c9c5bf"/>
  {#each [36, 44, 52, 60, 68, 76, 84] as x}<rect x={x} y="12" width="3" height="16" rx="1.5" fill="#8f8b85"/>{/each}
  <circle cx="60" cy="84" r="38" class="ring" data-mode={ring(3)} />
  <circle cx="60" cy="84" r="34" fill="url(#cap)" stroke="#bdb9b2" class="knob" class:sel={selectedIndex === 3} onclick={() => onPick(3)} role="button" tabindex="0" onkeydown={(e) => e.key === "Enter" && onPick(3)} />
  <rect x="24" y="130" width="72" height="156" rx="10" fill="#e4e1dc" stroke="#cdc9c3"/>
  {#each [0, 1, 2] as i}
    <circle cx="60" cy={158 + i * 50} r="23" class="ring" data-mode={ring(i)} />
    <circle cx="60" cy={158 + i * 50} r="20" fill="url(#cap)" stroke="#bdb9b2" class="btn" class:sel={selectedIndex === i} onclick={() => onPick(i)} role="button" tabindex="0" onkeydown={(e) => e.key === "Enter" && onPick(i)} />
    <text x="60" y={163 + i * 50} text-anchor="middle" font-size="14" fill="#6a6762">{["🎙", "✓", "✕"][i]}</text>
  {/each}
</svg>

<style>
  .device { display: block; margin: 0 auto; }
  .ring { fill: none; stroke: #e3e0db; stroke-width: 4; }
  .ring[data-mode="solid"] { stroke: var(--accent); }
  .ring[data-mode="breathe"] { stroke: var(--accent); animation: breathe 1.8s ease-in-out infinite; }
  .knob, .btn { cursor: pointer; }
  .knob.sel, .btn.sel { stroke: var(--accent); stroke-width: 3; }
  @keyframes breathe { 0%, 100% { opacity: .3; } 50% { opacity: 1; } }
</style>
```

- [ ] **Step 2: GestureCard.svelte 与 KnobLabel.svelte**

```svelte
<!-- GestureCard.svelte -->
<script lang="ts">
  import { t } from "../lib/i18n";
  import { actionLabel } from "../model/labels";
  import { effectivePress, effectiveDouble, effectiveLong, GESTURES, type ControlActionMap, type Gesture, type Halo } from "../model/keymap";
  let { index, control, selected, halos, onPick, layer, override }: { index: number; control: ControlActionMap; selected: Gesture | null; halos: Halo[]; onPick: (g: Gesture) => void; layer: number; override?: ControlActionMap } = $props();
  const inherited = (g: Gesture) => layer !== 0 && (!override || (override.inheritedGestures ?? []).includes(({ press: "press", double: "doublePress", long: "longPress" } as const)[g]));
  const text = (g: Gesture) => {
    if (inherited(g)) return "▽";
    const a = g === "press" ? effectivePress(control) : g === "double" ? effectiveDouble(control) : effectiveLong(control);
    if (g !== "press" && !a) return "⊠";
    return actionLabel(a, null, halos);
  };
  const names: Record<Gesture, string> = { press: "单击", double: "双击", long: "长按" };
</script>

<div class="card" class:sel={selected !== null}>
  {#each (index >= 4 ? ["press"] : GESTURES) as g}
    <button class="g" class:on={selected === g} onclick={() => onPick(g)}>
      <small>{t(names[g])}</small>
      <span class:mark={text(g) === "▽" || text(g) === "⊠"}>{text(g)}</span>
    </button>
  {/each}
</div>

<style>
  .card { background: var(--card); border-radius: var(--radius); padding: 10px 12px; display: flex; gap: 6px; border: 1px solid transparent; }
  .card.sel { border-color: #8a8a90; }
  .g { flex: 1; background: none; padding: 8px 4px; border-radius: 8px; display: flex; flex-direction: column; gap: 4px; align-items: center; }
  .g.on { background: var(--card-on); }
  .g small { color: var(--muted); font-size: 12px; }
  .g span { font-size: 13px; max-width: 110px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .g .mark { color: var(--muted); border: 1.5px solid var(--muted); border-radius: 4px; padding: 0 5px; font-size: 11px; }
</style>
```

```svelte
<!-- KnobLabel.svelte -->
<script lang="ts">
  import { t } from "../lib/i18n";
  let { index, label, selected, onPick }: { index: 4 | 5; label: string; selected: boolean; onPick: () => void } = $props();
</script>
<button class="knob" class:on={selected} onclick={onPick}>
  {#if index === 5}<i>↺</i>{/if}
  <span><small>{t(index === 4 ? "右旋" : "左旋")}</small>{label}</span>
  {#if index === 4}<i>↻</i>{/if}
</button>
<style>
  .knob { background: var(--card); border-radius: 8px; padding: 6px 10px; display: flex; align-items: center; gap: 8px; border: 1px solid transparent; }
  .knob.on { border-color: var(--accent); }
  .knob i { width: 30px; height: 30px; border-radius: 50%; border: 1.5px solid var(--muted); display: inline-flex; align-items: center; justify-content: center; font-style: normal; }
  .knob small { display: block; color: var(--muted); font-size: 12px; text-align: left; }
</style>
```

- [ ] **Step 3: LayerBar.svelte**

```svelte
<script lang="ts">
  import type { Snippet } from "svelte";
  let { layer, onSelect, right }: { layer: number; onSelect: (l: number) => void; right?: Snippet } = $props();
</script>
<div class="bar">
  <span class="muted">Layer</span>
  <div class="seg">{#each [0, 1, 2, 3] as l}<button class:on={layer === l} onclick={() => onSelect(l)}>{l}</button>{/each}</div>
  <div class="right">{@render right?.()}</div>
</div>
<style>
  .bar { display: flex; align-items: center; gap: 14px; padding: 12px 22px; }
  .muted { color: var(--muted); font-size: 13px; }
  .seg { display: inline-flex; background: var(--card); border-radius: 8px; padding: 3px; gap: 2px; }
  .seg button { padding: 5px 14px; background: none; font-family: var(--mono); }
  .seg button.on { background: var(--card-on); }
  .right { margin-left: auto; }
</style>
```

- [ ] **Step 4: Keymap.svelte 舞台（面板在 Task 8 接入）**

```svelte
<script lang="ts">
  import LayerBar from "../components/LayerBar.svelte";
  import DeviceView from "../components/DeviceView.svelte";
  import GestureCard from "../components/GestureCard.svelte";
  import KnobLabel from "../components/KnobLabel.svelte";
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  import { actionLabel, CONTROL_NAMES } from "../model/labels";
  import { effectivePress, type Gesture } from "../model/keymap";
  import { controlAt, resolvedControl, select, type Draft } from "../model/draft";

  let { draft = $bindable(), snapshot }: { draft: Draft; snapshot: UiSnapshot | null } = $props();
  const map = $derived(draft.map);
  const sel = $derived(draft.selection);
  const halos = $derived(map?.halos ?? []);
  const ctl = (i: number) => resolvedControl(map!, sel.layer, i);
  const pick = (index: number, gesture: Gesture) => { draft = select(draft, { layer: sel.layer, index, gesture }); };
  const lights = $derived(["off", "off", "off", "off"] as ("off" | "solid" | "breathe")[]); // 第 2 期接本层背光
</script>

{#if !map}
  <p class="empty">{t("未连接")}</p>
{:else}
  <LayerBar layer={sel.layer} onSelect={(l) => { draft = select(draft, { ...sel, layer: l }); }} />
  <section class="stage">
    <div class="side left">
      <div class="name">⟳ {t(CONTROL_NAMES[3])}</div>
      <GestureCard index={3} control={ctl(3)} selected={sel.index === 3 ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, 3)} onPick={(g) => pick(3, g)} />
      <div class="knobs"><KnobLabel index={5} label={actionLabel(effectivePress(ctl(5)), null, halos)} selected={sel.index === 5} onPick={() => pick(5, "press")} /></div>
    </div>
    <DeviceView {lights} selectedIndex={sel.index < 4 ? sel.index : null} onPick={(i) => pick(i, "press")} />
    <div class="side right">
      <div class="knobs end"><KnobLabel index={4} label={actionLabel(effectivePress(ctl(4)), null, halos)} selected={sel.index === 4} onPick={() => pick(4, "press")} /></div>
      {#each [0, 1, 2] as i}
        <GestureCard index={i} control={ctl(i)} selected={sel.index === i ? sel.gesture : null} {halos} layer={sel.layer} override={controlAt(map, sel.layer, i)} onPick={(g) => pick(i, g)} />
      {/each}
    </div>
  </section>
{/if}

<style>
  .empty { padding: 40px; color: var(--muted); text-align: center; }
  .stage { display: grid; grid-template-columns: 1fr 150px 1fr; gap: 18px; padding: 4px 40px 18px; align-items: center; }
  .side { display: flex; flex-direction: column; gap: 12px; justify-content: center; }
  .name { color: var(--muted); font-size: 14px; }
  .knobs { display: flex; } .knobs.end { justify-content: flex-end; }
</style>
```

- [ ] **Step 5: 运行** `npm run check && npm run build`，`cargo build -p halokey-app`，启动：Layer 切换、点卡片 / 设备图 / 旋钮标签能高亮，各卡片文字与 `docs/examples/halo-menu.json` 的内容一致。
- [ ] **Step 6: 提交** `git add ui && git commit -m "feat(ui): keymap stage with layer bar, device view and gesture cards"`。

---

### Task 8: 键码面板与分配

**Files:**
- Create: `ui/src/components/Palette.svelte`, `ui/src/components/KeyGrid.svelte`, `ui/src/components/OutputModeSegment.svelte`
- Modify: `ui/src/pages/Keymap.svelte`

**Interfaces:**
- `Palette` props：`{ draft: Draft (bindable); halos: Halo[]; apps: NamedHostAction[]; macros: NamedHostAction[] }`；内部用 Task 4 的 reducer 写回 `draft`。
- `KeyGrid` props：`{ keycaps: Keycap[]; current: KeyEntry[] | null; showTransparent: boolean; onPick: (entries: KeyEntry[]) => void; onClear: () => void; onTransparent: () => void }`。
- `OutputModeSegment` props：`{ behavior: Behavior; count: number; allowHold: boolean; onBehavior: (b) => void; onCount: (n) => void }`。

- [ ] **Step 1: OutputModeSegment.svelte**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import type { Behavior } from "../model/keymap";
  let { behavior, count, allowHold, onBehavior, onCount }: { behavior: Behavior; count: number; allowHold: boolean; onBehavior: (b: Behavior) => void; onCount: (n: number) => void } = $props();
</script>
<div class="seg">
  {#if allowHold}<button class:on={behavior === "hold"} onclick={() => onBehavior("hold")}>{t("保持按住")}</button>{/if}
  <button class:on={behavior === "tap"} onclick={() => onBehavior("tap")}>{t("短按一次")}</button>
  <button class:on={behavior === "burst"} onclick={() => onBehavior("burst")}>{t("连按")}</button>
  {#if behavior === "burst"}
    <span class="count"><button onclick={() => onCount(count - 1)}>−</button>{count} {t("次")}<button onclick={() => onCount(count + 1)}>+</button></span>
  {/if}
</div>
<style>
  .seg { display: inline-flex; background: var(--card); border-radius: 8px; padding: 3px; gap: 2px; align-items: center; }
  .seg > button { background: none; padding: 5px 12px; font-size: 13px; }
  .seg > button.on { background: var(--accent); color: var(--accent-text); }
  .count { display: inline-flex; align-items: center; gap: 6px; padding: 0 6px; font-family: var(--mono); font-size: 13px; }
  .count button { padding: 2px 8px; }
</style>
```

- [ ] **Step 2: KeyGrid.svelte**

```svelte
<script lang="ts">
  import type { Keycap } from "../model/codes";
  import type { KeyEntry } from "../model/keymap";
  let { keycaps, current, showTransparent, onPick, onClear, onTransparent }: { keycaps: Keycap[]; current: KeyEntry[] | null; showTransparent: boolean; onPick: (entries: KeyEntry[]) => void; onClear: () => void; onTransparent: () => void } = $props();
  const isOn = (k: Keycap) => !!current && current.length === 1 && current[0]!.type === k.entry.type && current[0]!.code === k.entry.code;
</script>
<div class="grid">
  {#if showTransparent}<button class="special" title="透明继承" onclick={onTransparent}>▽</button>{/if}
  <button class="special" title="不执行动作 / 关闭手势" onclick={onClear}>⊠</button>
  {#each keycaps as k}<button class:on={isOn(k)} onclick={() => onPick([k.entry])}>{k.label}</button>{/each}
</div>
<style>
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(78px, 1fr)); gap: 8px; }
  .grid button { height: 44px; background: var(--card); font-size: 13px; line-height: 1.15; padding: 4px; }
  .grid button.on { background: var(--accent); color: var(--accent-text); }
  .grid .special { color: var(--muted); }
</style>
```

- [ ] **Step 3: Palette.svelte**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  import KeyGrid from "./KeyGrid.svelte";
  import OutputModeSegment from "./OutputModeSegment.svelte";
  import { CATEGORIES, type CategoryId } from "../model/codes";
  import { CONTROL_NAMES } from "../model/labels";
  import { isRotation, type Behavior, type Halo, type KeyEntry, type NamedHostAction } from "../model/keymap";
  import { assignAction, assignEntries, clearGesture, resolvedControl, setBehavior, setTapCount, setTransparent, type Draft } from "../model/draft";

  let { draft = $bindable(), halos, apps, macros }: { draft: Draft; halos: Halo[]; apps: NamedHostAction[]; macros: NamedHostAction[] } = $props();
  let category = $state<CategoryId>("common");
  let step = $state(20);
  const sel = $derived(draft.selection);
  const control = $derived(draft.map ? resolvedControl(draft.map, sel.layer, sel.index) : null);
  const gestureName: Record<string, string> = { press: "单击", double: "双击", long: "长按" };
  const current = $derived<KeyEntry[] | null>(!control ? null : sel.gesture === "press" ? (control.pressAction ? null : control.press) : sel.gesture === "double" ? (control.doublePressAction ? null : control.doublePress ?? null) : (control.longPressAction ? null : control.longPress ?? null));
  const behavior = $derived<Behavior>(!control ? "hold" : sel.gesture === "press" ? control.pressBehavior ?? "hold" : sel.gesture === "double" ? control.doublePressBehavior ?? "tap" : control.longPressBehavior ?? "hold");
  const count = $derived(!control ? 2 : sel.gesture === "press" ? control.pressTapCount ?? 2 : sel.gesture === "double" ? control.doublePressTapCount ?? 2 : control.longPressTapCount ?? 2);
  const rotation = $derived(isRotation(sel.index));
  const cat = $derived(CATEGORIES.find((c) => c.id === category)!);
  const layerAllowed = $derived(!rotation && sel.gesture !== "double");
  const pointer = (dx: number, dy: number) => { draft = assignAction(draft, { pointerMove: { _0: { dx, dy } } }); };
</script>

<section class="panel">
  <nav class="cats">
    {#each CATEGORIES as c, i}
      {#if i === 5}<hr>{/if}
      <button class:on={category === c.id} disabled={c.id === "layer" && !layerAllowed} onclick={() => (category = c.id)}>{t(c.name)}</button>
    {/each}
  </nav>
  <div class="pal">
    <div class="head">
      <span class="crumb">{t(CONTROL_NAMES[sel.index] ?? "")} › {t(gestureName[sel.gesture] ?? "")}</span>
      <div class="mode"><OutputModeSegment {behavior} {count} allowHold={!rotation && sel.gesture !== "double"} onBehavior={(b) => (draft = setBehavior(draft, b))} onCount={(n) => (draft = setTapCount(draft, n))} /></div>
    </div>
    {#if cat.keycaps}
      <KeyGrid keycaps={cat.keycaps} {current} showTransparent={sel.layer !== 0} onPick={(e) => (draft = assignEntries(draft, e))} onClear={() => (draft = clearGesture(draft))} onTransparent={() => (draft = setTransparent(draft))} />
    {:else if category === "layer"}
      <div class="cards">
        {#if sel.layer !== 0}<button class="special" onclick={() => (draft = setTransparent(draft))}>▽</button>{/if}
        <button class="special" onclick={() => (draft = clearGesture(draft))}>⊠</button>
        {#each [1, 2, 3] as l}<button onclick={() => (draft = assignAction(draft, { momentaryLayer: { _0: l } }))}>MO({l})</button>{/each}
      </div>
    {:else if category === "pointer"}
      <div class="cards">
        <button onclick={() => pointer(0, -step)}>↑</button><button onclick={() => pointer(0, step)}>↓</button>
        <button onclick={() => pointer(-step, 0)}>←</button><button onclick={() => pointer(step, 0)}>→</button>
        <label>{t("指针步长")} <input type="number" min="1" max="1000" bind:value={step}> {t("像素")}</label>
      </div>
    {:else if category === "halo"}
      <div class="cards">
        {#each halos as h, i}<button class="item" onclick={() => (draft = assignAction(draft, { halo: { _0: h.id } }))}><small>◎ H{i}</small>{h.name}</button>{/each}
        {#if !halos.length}<span class="muted">{t("在本机构建里还没有的功能")}</span>{/if}
      </div>
    {:else if category === "app" || category === "macro"}
      <div class="cards">
        {#each (category === "app" ? apps : macros) as item}<button class="item" onclick={() => (draft = assignAction(draft, { library: { _0: item.id } }))}><small>{category === "app" ? "A" : "M"}{item.slot}</small>{item.name}</button>{/each}
        <span class="muted">{t("在本机构建里还没有的功能")}</span>
      </div>
    {:else}
      <span class="muted">{t("在本机构建里还没有的功能")}</span>
    {/if}
  </div>
</section>

<style>
  .panel { display: grid; grid-template-columns: 150px 1fr; border-top: 1px solid var(--line); min-height: 280px; }
  .cats { padding: 14px 12px; display: flex; flex-direction: column; gap: 2px; border-right: 1px solid var(--line); }
  .cats button { text-align: left; background: none; padding: 7px 10px; color: #c9c9ce; }
  .cats button.on { background: var(--card-on); color: var(--text); }
  .cats hr { border: 0; border-top: 1px solid var(--line); margin: 8px 0; }
  .pal { padding: 14px 18px; }
  .head { display: flex; align-items: center; gap: 10px; margin-bottom: 12px; }
  .crumb { font-size: 15px; }
  .mode { margin-left: auto; }
  .cards { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
  .cards button { min-width: 86px; height: 44px; }
  .cards .item { display: flex; flex-direction: column; align-items: center; gap: 2px; width: 150px; height: 60px; }
  .cards .item small { color: var(--muted); font-family: var(--mono); font-size: 11px; }
  .cards .special { color: var(--muted); }
  .muted { color: var(--muted); font-size: 13px; }
</style>
```

- [ ] **Step 4: 接进 Keymap.svelte**

在舞台 `</section>` 后加：

```svelte
  <Palette bind:draft {halos} apps={(map.actionLibrary ?? []).filter((a) => "application" in a.action)} macros={(map.actionLibrary ?? []).filter((a) => "macro" in a.action)} />
```

并 `import Palette from "../components/Palette.svelte";`。

- [ ] **Step 5: 运行** `npm run check && npm test && npm run build`，启动应用：选"顶部按键 › 单击"后点 `Enter`，卡片立即显示 Enter、顶栏出现未保存圆点；旋转手势只显示单击，组合键 / Layer 的 hold 按钮按规则禁用；指针分类能分配步长。
- [ ] **Step 6: 提交** `git add ui && git commit -m "feat(ui): palette with keycap grid, layer, pointer and output modes"`。

---

### Task 9: 保存 / 撤销 + Rust `validate_keymap` + `activeLayers`

**Files:**
- Create: `ui/src/components/BottomBar.svelte`
- Modify: `ui/src/pages/Keymap.svelte`, `apps/halokey/src/main.rs`, `crates/runtime/src/lib.rs`
- Test: `crates/runtime` 无新测试（`active_layers` 由 host 的已有测试覆盖）；前端 `ui/src/lib/__tests__/tauri.test.ts` 已覆盖 mock 校验。

**Interfaces:**
- Rust 命令 `validate_keymap(keymap: serde_json::Value) -> Result<(), String>`：只调 `HostKeymap::from_json`，不写文件。
- `Snapshot.active_layers: Vec<u8>`；`UiSnapshot.active_layers`（serde camelCase → `activeLayers`）。
- `BottomBar` props：`{ dirty: boolean; saving: boolean; message: string; error: boolean; onSave: () => void; onRevert: () => void }`。

- [ ] **Step 1: Rust：validate_keymap 与 active_layers**

`crates/runtime/src/lib.rs`：`Snapshot` 加字段 `pub active_layers: Vec<u8>,`（`Default` 派生自动补空向量）；`apply()` 里有两处提前 `return`（空 transitions、没有 injector），所以要插在 `(self.on_event)(Event::Transitions(transitions.clone()));` 这一行之后、`let Some(mut injector)` 之前：

```rust
        let layers = self.router.active_layers();
        self.update(|s| s.active_layers = layers);
```

`Snapshot` 若没有派生 `Default` 而是手写初始化，就在那处加 `active_layers: Vec::new()`。

`apps/halokey/src/main.rs`：`UiSnapshot` 加 `active_layers: Vec<u8>`，`from` 里 `active_layers: s.active_layers,`；新增命令并注册到 `generate_handler!`：

```rust
/// 只校验不保存，供界面实时显示错误。
#[tauri::command]
fn validate_keymap(keymap: serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(&keymap).map_err(|e| e.to_string())?;
    HostKeymap::from_json(&bytes).map(|_| ()).map_err(|e| e.to_string())
}
```

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` → 绿。

- [ ] **Step 2: BottomBar.svelte**

```svelte
<script lang="ts">
  import { t } from "../lib/i18n";
  let { dirty, saving, message, error, onSave, onRevert }: { dirty: boolean; saving: boolean; message: string; error: boolean; onSave: () => void; onRevert: () => void } = $props();
</script>
<footer>
  <span class="msg" class:error>{message}</span>
  <button disabled={!dirty || saving} onclick={onRevert}>{t("撤销更改")}</button>
  <button class="primary" disabled={!dirty || saving} onclick={onSave}>{t("保存到本机")}</button>
</footer>
<style>
  footer { display: flex; justify-content: flex-end; align-items: center; gap: 10px; padding: 12px 22px; border-top: 1px solid var(--line); position: sticky; bottom: 0; background: var(--bg); }
  .msg { margin-right: auto; color: var(--muted); font-size: 13px; }
  .msg.error { color: var(--bad); }
</style>
```

- [ ] **Step 3: Keymap.svelte 接保存**

```svelte
  import BottomBar from "../components/BottomBar.svelte";
  import { api } from "../lib/tauri";
  import { markSaved, revert } from "../model/draft";
  let saving = $state(false); let message = $state(""); let error = $state(false);
  async function save() {
    if (!draft.map) return;
    saving = true; error = false; message = "";
    try { await api.validateKeymap(draft.map); await api.saveKeymap(draft.map); draft = markSaved(draft); message = t("已保存"); }
    catch (e) { error = true; message = String(e); }
    finally { saving = false; }
  }
```

模板末尾：`<BottomBar dirty={draft.dirty} {saving} {message} {error} onSave={save} onRevert={() => { draft = revert(draft); message = ""; }} />`。

- [ ] **Step 4: 真机验证**

启动应用：改键 1 单击为 `Tab` → 保存 → 按键 1 打出 Tab；把旋转的输出方式改错是不可能的（按钮被禁），但手动在 JSON 里把 `doublePressBehavior` 改成 `hold` 后导入（第 5 期）才会触发 Rust 错误；此处用 mock 测试 `validateKeymap` 的 reject 路径即可。拔掉接收器后点保存：底栏显示"已保存"，插回后新键位生效。

- [ ] **Step 5: 提交** `git add -A && git commit -m "feat(ui,app,runtime): save and revert with Rust-side validation; expose active layers"`。

---

### Task 10: 设置页（只读 + 已有的三个写入项）

**Files:**
- Modify: `ui/src/pages/Settings.svelte`

**Interfaces:** 使用 `api.deviceInfo / powerInfo / getSettings / setIdleMinutes / getAutostart / setAutostart`，以及 `setLang`。

- [ ] **Step 1: 实现**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { t, lang, setLang } from "../lib/i18n";
  import { api, type UiSnapshot, type UiDeviceInfo, type UiPower } from "../lib/tauri";
  let { snapshot }: { snapshot: UiSnapshot | null } = $props();
  let info = $state<UiDeviceInfo | null>(null);
  let power = $state<UiPower | null>(null);
  let idle = $state(5);
  let autostart = $state(false);
  let note = $state("");
  const IDLE = [0, 1, 5, 10, 15, 30, 60];
  const modeName = ["全灭", "全亮", "工作"];
  async function load() {
    try { [info, power] = await Promise.all([api.deviceInfo(), api.powerInfo()]); } catch (e) { note = String(e); }
    try { idle = (await api.getSettings()).idleMinutes; autostart = await api.getAutostart(); } catch { /* mock */ }
  }
  onMount(load);
  const sleepText = (s: number | null) => s == null ? "—" : s === 0 ? t("永不") : `${s / 3600} h`;
</script>

<div class="page">
  <div class="ph"><h2>{t("设置")}</h2><label class="lang">🌐 {t("界面语言")} <select value={lang()} onchange={(e) => setLang((e.target as HTMLSelectElement).value as "zh" | "en")}><option value="zh">简体中文</option><option value="en">English</option></select></label></div>
  <p class="muted">{t("第 1 期：设置页只读；写入项在后续计划中")}</p>

  <section class="card">
    <h3>{t("Vibe Key")} <span class="r"><button onclick={load}>{t("刷新状态")}</button></span></h3>
    <div class="row"><span class="k">{t("状态")}</span><span class="v">{snapshot?.connected ? "● " + (snapshot.online === false ? t("本体离线") : t("本体在线")) : "○ " + t("未连接")}{#if snapshot?.battery} · 🔋 {snapshot.battery[0] ?? "?"}% · {snapshot.battery[1]} mV{/if}</span><span class="h">{t("心跳")} {snapshot?.heartbeats ?? 0} · {t("回显")} {snapshot?.echoes ?? 0}</span></div>
    <div class="row"><span class="k">{t("固件")}</span><span class="v mono">{info?.firmware ?? "—"} / {info?.dongleFirmware ?? "—"}</span></div>
    <div class="row"><span class="k">{t("空闲后交还控制")}</span><span class="v"><select value={idle} onchange={async (e) => { idle = Number((e.target as HTMLSelectElement).value); await api.setIdleMinutes(idle); }}>{#each IDLE as m}<option value={m}>{m === 0 ? t("永不") : `${m} ${t("分钟")}`}</option>{/each}</select></span></div>
  </section>

  <div class="two">
    <section class="card">
      <h3>{t("灯光")}</h3>
      <div class="row"><span class="k">{t("灯光模式")}</span><span class="v">{info?.lighting ? t(modeName[info.lighting.mode] ?? "—") : "—"}</span></div>
      <div class="row"><span class="k">{t("全亮亮度")}</span><span class="v mono">{info?.lighting?.brightness ?? "—"} / 20</span></div>
      <div class="row"><span class="k"></span><span class="v muted">{t("各灯效果在键位页按 Layer 设置")}</span></div>
    </section>
    <section class="card">
      <h3>{t("麦克风")}</h3>
      <div class="row"><span class="k">{t("收音")}</span><span class="v">{power?.micEnabled == null ? "—" : power.micEnabled ? "✓" : "✕"}</span></div>
      <div class="row"><span class="k">{t("硬件降噪")}</span><span class="v">{power?.noiseReduction == null ? "—" : power.noiseReduction ? "✓" : "✕"}</span></div>
    </section>
  </div>

  <div class="two">
    <section class="card">
      <h3>{t("电源")}</h3>
      <div class="row"><span class="k">{t("休眠 / 自动关机")}</span><span class="v">{sleepText(power?.sleepDelay ?? null)}</span></div>
      <div class="row"><span class="k">{t("待机延时")}</span><span class="v mono">{power?.standbyDelay ?? "—"} s</span></div>
    </section>
    <section class="card">
      <h3>{t("通用")}</h3>
      <div class="row"><span class="k">{t("登录时启动")}</span><span class="v"><input type="checkbox" bind:checked={autostart} onchange={async () => { try { autostart = await api.setAutostart(autostart); } catch (e) { note = String(e); } }}></span></div>
      <div class="row"><span class="k">{t("当前文件")}</span><span class="v mono">{snapshot?.configPath ?? "—"}</span></div>
      <div class="row"><span class="k">{t("关于")}</span><span class="v mono">halokey 0.0.1</span></div>
    </section>
  </div>
  {#if note}<p class="bad">{note}</p>{/if}
</div>

<style>
  .page { padding: 18px 22px; display: flex; flex-direction: column; gap: 14px; }
  .ph { display: flex; align-items: center; } .ph h2 { margin: 0; font-size: 18px; } .lang { margin-left: auto; color: var(--muted); }
  .card { background: var(--panel); border-radius: 12px; padding: 14px 18px; }
  .card h3 { margin: 0 0 10px; font-size: 14px; display: flex; align-items: center; } .card h3 .r { margin-left: auto; }
  .row { display: flex; align-items: center; gap: 14px; padding: 8px 0; border-top: 1px solid var(--line); }
  .row:first-of-type { border-top: 0; }
  .k { width: 140px; color: #c9c9ce; } .v { display: flex; align-items: center; gap: 10px; flex: 1; } .h { color: var(--muted); font-size: 12.5px; margin-left: auto; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .mono { font-family: var(--mono); font-size: 12.5px; }
  .muted { color: var(--muted); font-size: 13px; } .bad { color: var(--bad); }
</style>
```

`t("心跳")` / `t("回显")` 已在 Task 6 的 `EN` 表里；若再新增字符串，Task 6 的扫描测试会指出缺的那条。

- [ ] **Step 2: 运行** `npm test && npm run check && npm run build`，启动应用：设置页显示固件、灯光模式、电源值，改空闲分钟数后 `~/.config/halokey/settings.json` 更新，切英文后两页都变英文。
- [ ] **Step 3: 提交** `git add ui && git commit -m "feat(ui): read-only settings page with idle, autostart and language"`。

---

### Task 11: 清理旧前端、CI、文档

**Files:**
- Delete: `ui/legacy/`
- Modify: `README.md`, `README.zh.md`（"现在就能试"一节加 `cd ui && npm install` 前置说明；界面截图下一期补），`docs/01-architecture.md` + `.zh.md`（第 3 条里程碑追加"第 1 期前端重做完成：Svelte 5 + Vite"），`AGENTS.md` §3 技术栈加前端一行。

- [ ] **Step 1: 删除并更新文档**

```bash
git rm -r ui/legacy
```

README 两版在 `cargo run -p halokey-app` 之前各加一行，英文：`cd ui && npm install && cd ..   # once; the Tauri build runs the Vite build itself`，中文：`cd ui && npm install && cd ..   # 只需一次；Tauri 构建会自己跑 Vite`。

- [ ] **Step 2: 全量检查**

Run: `python3 tools/check_docs.py && cd ui && npm test && npm run check && npm run build && cd .. && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --workspace`
Expected: 全绿。

- [ ] **Step 3: 真机验收清单（在本机逐项做，记录到提交信息）**

1. 托盘图标 → 设置窗口打开，顶栏连接状态与电量正确。
2. 键位页：四层切换；点设备图上的键 / 旋钮 / 卡片 / 旋钮标签都能选中；卡片文字与 JSON 一致。
3. 把键 2 单击改成 `Tab`，保存，按键 2 打出 Tab；撤销后恢复。
4. Layer 1 给键 3 长按分配 `Esc`：卡片显示 ▽ / 具体值正确；保存后 JSON 的 `layers` 出现覆盖记录；按住 MO 键再长按键 3（需要先在 Layer 0 配 MO(1)）输出 Esc。
5. 指针分类：旋钮右旋分配 →20，保存，拧旋钮指针右移。
6. 光环分类：旋钮按下分配 H0，保存，按旋钮出圆泡转盘，能选能确认。
7. 设置页：固件、灯光模式、电源、麦克风值与 `halokey status` / 设备信息一致；空闲分钟数修改生效；英文切换无漏翻。
8. 拔插接收器：顶栏状态变化；重连后键位不被草稿覆盖（先改一个键不保存，再拔插）。
9. CI：推送后三平台绿。

- [ ] **Step 4: 提交并推送**

```bash
git add -A && git commit -m "chore(ui): remove the legacy settings page; phase 1 of the redesign complete

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" && git push origin main
```

---

## 自检记录

- 规格覆盖：§2 技术栈（Task 1）、§3 信息架构与顶栏（Task 6）、§4.1 Layer 条（Task 7；背光条留第 2 期）、§4.2 设备图（Task 7）、§4.3 卡片（Task 7）、§4.4 面板（Task 8；组合键录制 / APP / 宏 / 光环的新建编辑器属第 3–4 期，本期只做"选已有项"）、§4.5 保存撤销（Task 9）、§6 设置页只读（Task 10）、§8 `validate_keymap` 与 `activeLayers`（Task 9）、§11 清理（Task 11）。
- 类型一致性：`Draft` / `Selection` / `Gesture` / `Behavior` 在 Task 2、4、7、8 中同名；`api.validateKeymap` / `saveKeymap` 在 Task 5、9 中同名；`UiSnapshot.activeLayers` 在 Task 5、9 对应 Rust 的 `active_layers`。
- Review Focus 五项分别落在 Task 2（未知动作往返）、Task 4（dirty 不覆盖）、Task 4 + 8（旋转限制）、Task 9（断开时保存）、Task 6（i18n 扫描）。
