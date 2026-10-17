<script lang="ts">
  import { t } from "../lib/i18n";
  import type { UiSnapshot } from "../lib/tauri";
  import type { Draft } from "../model/draft";
  let { draft = $bindable(), snapshot }: { draft: Draft; snapshot: UiSnapshot | null } = $props();
</script>
<p>{t("在本机构建里还没有的功能")} {draft.dirty ? "*" : ""} {snapshot?.connected ? "" : ""}</p>
