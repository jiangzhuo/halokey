<script lang="ts">
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  let { snapshot }: { snapshot: UiSnapshot | null } = $props();
</script>
<p>{t("在本机构建里还没有的功能")} {snapshot?.connected ? "" : ""}</p>
