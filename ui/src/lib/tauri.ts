// Task 1 的最小版本，Task 5 会替换为类型化的完整桥。
export type UiEvent = { kind: string } & Record<string, any>;
export async function subscribe(handler: (e: UiEvent) => void): Promise<() => void> {
  const w = window as any;
  if (!w.__TAURI_INTERNALS__) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<UiEvent>("halokey", (ev) => handler(ev.payload));
  return un;
}
