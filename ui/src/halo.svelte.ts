import { mount } from "svelte";
import HaloWheel from "./components/HaloWheel.svelte";
import { subscribe, type UiEvent } from "./lib/tauri";
import type { Halo } from "./model/keymap";

// 转盘窗口：状态只来自运行时事件；没有光环时什么都不画（窗口透明）。
const state = $state({ halo: null as Halo | null, selected: 0, size: 360 });
mount(HaloWheel, { target: document.body, props: state });
subscribe((e: UiEvent) => {
  if (e.kind === "haloShow") { state.halo = e.halo; state.selected = e.selected; state.size = e.size ?? 360; }
  else if (e.kind === "haloSelect") state.selected = e.selected;
  else if (e.kind === "haloHide") state.halo = null;
});
