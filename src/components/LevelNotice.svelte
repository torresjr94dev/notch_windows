<script lang="ts">
  import Sun from "@lucide/svelte/icons/sun";
  import SunDim from "@lucide/svelte/icons/sun-dim";
  import Volume from "@lucide/svelte/icons/volume";
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";

  interface Props {
    kind: "volume" | "brightness";
    /** 0 a 1. */
    level: number;
    muted?: boolean;
  }

  let { kind, level, muted = false }: Props = $props();

  const percent = $derived(Math.round(level * 100));
  const silent = $derived(kind === "volume" && (muted || percent === 0));
  const label = $derived(
    kind === "volume"
      ? silent
        ? "Volumen silenciado"
        : `Volumen ${percent}%`
      : `Brillo ${percent}%`,
  );
  const Icon = $derived(
    kind === "brightness"
      ? level < 0.4
        ? SunDim
        : Sun
      : silent
        ? VolumeX
        : level < 0.34
          ? Volume
          : level < 0.67
            ? Volume1
            : Volume2,
  );
</script>

<div class="notice" class:silent role="status" aria-label={label}>
  <Icon size={18} strokeWidth={2.25} aria-hidden="true" />
  <span class="track">
    <span class="fill" style:transform="scaleX({silent ? 0 : level})"></span>
  </span>
  <span class="value">{silent ? "—" : percent}</span>
</div>

<style>
  .notice {
    width: 100%;
    height: 100%;
    padding: 0 16px 0 14px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .track {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: var(--track);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--text-primary);
    transform-origin: left;
    transition: transform 140ms ease-out;
  }

  .value {
    width: 26px;
    text-align: right;
    font-size: 13px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .silent {
    color: var(--text-secondary);
  }
</style>
