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
