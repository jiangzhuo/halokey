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
  footer { display: flex; justify-content: flex-end; align-items: center; gap: 10px; padding: 12px 22px; border-top: 1px solid var(--line); background: var(--bg); flex: none; }
  .msg { margin-right: auto; color: var(--muted); font-size: 13px; }
  .msg.error { color: var(--bad); }
</style>
