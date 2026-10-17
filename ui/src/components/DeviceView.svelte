<script lang="ts">
  let { lights, selectedIndex, onPick }: { lights: ("off" | "solid" | "breathe")[]; selectedIndex: number | null; onPick: (index: number) => void } = $props();
  const ring = (i: number) => lights[i] ?? "off";
</script>

<svg viewBox="0 0 120 300" width="118" height="295" class="device" role="img" aria-label="Vibe Key">
  <defs>
    <linearGradient id="body" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#f1efea"/><stop offset="1" stop-color="#d9d6d0"/></linearGradient>
    <radialGradient id="cap" cx=".4" cy=".35"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#cfccc6"/></radialGradient>
  </defs>
  <rect x="6" y="4" width="108" height="292" rx="24" fill="url(#body)" stroke="#c9c5bf"/>
  {#each [36, 44, 52, 60, 68, 76, 84] as x}<rect {x} y="12" width="3" height="16" rx="1.5" fill="#8f8b85"/>{/each}
  <circle cx="60" cy="84" r="38" class="ring" data-mode={ring(3)} />
  <circle cx="60" cy="84" r="34" fill="url(#cap)" stroke="#bdb9b2" class="knob" class:sel={selectedIndex === 3} onclick={() => onPick(3)} role="button" tabindex="0" onkeydown={(e) => e.key === "Enter" && onPick(3)} />
  <rect x="24" y="130" width="72" height="156" rx="10" fill="#e4e1dc" stroke="#cdc9c3"/>
  {#each [0, 1, 2] as i}
    <circle cx="60" cy={158 + i * 50} r="23" class="ring" data-mode={ring(i)} />
    <circle cx="60" cy={158 + i * 50} r="20" fill="url(#cap)" stroke="#bdb9b2" class="btn" class:sel={selectedIndex === i} onclick={() => onPick(i)} role="button" tabindex="0" onkeydown={(e) => e.key === "Enter" && onPick(i)} />
    <text x="60" y={163 + i * 50} text-anchor="middle" font-size="14" fill="#6a6762" pointer-events="none">{["🎙", "✓", "✕"][i]}</text>
  {/each}
</svg>

<style>
  .device { display: block; margin: 0 auto; }
  .ring { fill: none; stroke: #e3e0db; stroke-width: 4; }
  .ring[data-mode="solid"] { stroke: var(--accent); }
  .ring[data-mode="breathe"] { stroke: var(--accent); animation: breathe 1.8s ease-in-out infinite; }
  .knob, .btn { cursor: pointer; }
  .knob.sel, .btn.sel { stroke: var(--accent); stroke-width: 3; }
  @keyframes breathe { 0%, 100% { opacity: .3; } 50% { opacity: 1; } }
</style>
