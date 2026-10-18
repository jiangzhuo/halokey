<script lang="ts">
  import { t } from "../lib/i18n";
  import { actionLabel } from "../model/labels";
  import { effectivePress, effectiveDouble, effectiveLong, GESTURES, type ControlActionMap, type Gesture, type Halo, type NamedHostAction } from "../model/keymap";
  let { index, control, selected, halos, library = [], onPick, layer, override }: { index: number; control: ControlActionMap; selected: Gesture | null; halos: Halo[]; library?: NamedHostAction[]; onPick: (g: Gesture) => void; layer: number; override?: ControlActionMap } = $props();
  const GKEY = { press: "press", double: "doublePress", long: "longPress" } as const;
  const inherited = (g: Gesture) => layer !== 0 && (!override || (override.inheritedGestures ?? []).includes(GKEY[g]));
  const text = (g: Gesture) => {
    if (inherited(g)) return "▽";
    const a = g === "press" ? effectivePress(control) : g === "double" ? effectiveDouble(control) : effectiveLong(control);
    if (g !== "press" && !a) return "⊠";
    return actionLabel(a, null, halos, library);
  };
  const names: Record<Gesture, string> = { press: "单击", double: "双击", long: "长按" };
  const gestures = $derived(index >= 4 ? (["press"] as Gesture[]) : [...GESTURES]);
</script>

<div class="card" class:sel={selected !== null}>
  {#each gestures as g}
    <button class="g" class:on={selected === g} onclick={() => onPick(g)}>
      <small>{t(names[g])}</small>
      <span class:mark={text(g) === "▽" || text(g) === "⊠"}>{text(g)}</span>
    </button>
  {/each}
</div>

<style>
  .card { background: var(--card); border-radius: var(--radius); padding: 8px 10px; display: flex; gap: 6px; border: 1px solid transparent; height: 72px; }
  .card.sel { border-color: var(--accent); }
  .g { flex: 1; background: none; padding: 6px 4px; border-radius: var(--r-md); display: flex; flex-direction: column; gap: 4px; align-items: center; justify-content: center; }
  .g:hover { background: var(--card-on); }
  .g.on { background: var(--card-on); }
  .g small { color: var(--muted); font-size: 12px; white-space: nowrap; }
  .g span { font-size: 13px; max-width: 110px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .g .mark { color: var(--muted); font-size: 13px; }
</style>
