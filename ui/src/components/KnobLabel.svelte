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
