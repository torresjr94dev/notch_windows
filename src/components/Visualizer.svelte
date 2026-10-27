<script lang="ts">
  import { groupBands } from "../lib/bands";

  interface Props {
    /** Niveles 0–255 por banda, como los emite Rust. */
    levels: number[];
    bars: number;
    height: number;
    color: string;
    /** En pausa las barras se quedan en su mínimo. */
    active?: boolean;
  }

  let { levels, bars, height, color, active = true }: Props = $props();

  const MIN_SCALE = 0.14;
  const scales = $derived(
    groupBands(levels, bars).map((v) => (active ? Math.max(MIN_SCALE, v / 255) : MIN_SCALE)),
  );
</script>

<div class="visualizer" style:height="{height}px" style:--bar-color={color} aria-hidden="true">
  {#each scales as scale, i (i)}
    <span class="bar" style:transform="scaleY({scale})"></span>
  {/each}
</div>

<style>
  .visualizer {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  /* Solo `transform`: se compone en GPU sin recalcular layout 30 veces por segundo. La
     transición corta rellena los cuadros entre eventos de Rust (~33 ms). */
  .bar {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: var(--bar-color);
    transition: transform 70ms linear;
  }
</style>
