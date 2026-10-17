import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  // 组件测试在 jsdom 里 mount，需要走 svelte 的 browser 入口。
  resolve: { conditions: ["browser"] },
  test: { environment: "jsdom", include: ["src/**/*.test.ts", "src/**/*.test.svelte.ts"] },
});
