<script lang="ts">
  /** 一套 16px 线性图标（描边 currentColor），替代散落的 emoji 与文字符号。 */
  let { name, size = 16, level }: { name: string; size?: number; level?: number | null } = $props();
  const sw = 1.75;
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={sw} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === "keyboard"}
    <rect x="2" y="6" width="20" height="12" rx="2" /><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" />
  {:else if name === "settings"}
    <circle cx="12" cy="12" r="3" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1" />
  {:else if name === "battery"}
    <rect x="2" y="7" width="18" height="10" rx="2" /><path d="M22 10v4" />
    {#if level != null}<rect x="4" y="9" width={Math.max(1, 14 * Math.min(1, Math.max(0, level / 100)))} height="6" rx="1" fill="currentColor" stroke="none" />{/if}
  {:else if name === "star" || name === "common"}
    <path d="m12 3 2.7 5.8 6.3.7-4.7 4.3 1.3 6.2L12 16.9 6.4 20l1.3-6.2L3 9.5l6.3-.7z" />
  {:else if name === "letters"}
    <text x="12" y="16.5" text-anchor="middle" font-size="13" font-weight="600" fill="currentColor" stroke="none" font-family="inherit">Aa</text>
  {:else if name === "fkeys"}
    <text x="12" y="16.5" text-anchor="middle" font-size="12" font-weight="600" fill="currentColor" stroke="none" font-family="inherit">F1</text>
  {:else if name === "keypad"}
    <path d="M6 6h.01M12 6h.01M18 6h.01M6 12h.01M12 12h.01M18 12h.01M6 18h.01M12 18h.01M18 18h.01" stroke-width="3" />
  {:else if name === "mouse"}
    <rect x="7" y="2" width="10" height="20" rx="5" /><path d="M12 2v7" />
  {:else if name === "combo"}
    <text x="12" y="17" text-anchor="middle" font-size="16" fill="currentColor" stroke="none" font-family="inherit">⌘</text>
  {:else if name === "layer"}
    <path d="m12 3 9 5-9 5-9-5z" /><path d="m3 13 9 5 9-5" />
  {:else if name === "app" || name === "window"}
    <rect x="3" y="4" width="18" height="16" rx="3" /><path d="M3 9h18" />{#if name === "window"}<path d="M6.5 6.5h.01" stroke-width="3" />{/if}
  {:else if name === "macro"}
    <path d="M4 7h16M4 12h16M4 17h10" />
  {:else if name === "halo"}
    <circle cx="12" cy="12" r="8" /><circle cx="12" cy="12" r="2" fill="currentColor" stroke="none" />
  {:else if name === "pointer"}
    <path d="M5 4l7 15 2-6 6-2z" />
  {:else if name === "clock"}
    <circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" />
  {:else if name === "globe"}
    <circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" />
  {/if}
</svg>

<style>
  .icon { display: inline-block; vertical-align: -3px; flex: none; }
</style>
