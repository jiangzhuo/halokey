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
