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
  .seg { display: inline-flex; background: var(--card); border-radius: var(--r-md); padding: 3px; gap: 2px; align-items: center; }
  .seg > button { background: none; padding: 5px 12px; font-size: 13px; }
  .seg > button:hover { background: var(--card-on); }
  .seg > button.on { background: var(--card-on); color: var(--accent); }
  .count { display: inline-flex; align-items: center; gap: 6px; padding: 0 6px; font-family: var(--mono); font-size: 13px; }
  .count button { padding: 2px 8px; }
</style>
