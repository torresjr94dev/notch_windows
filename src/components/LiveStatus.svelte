<script lang="ts">
  import CircleAlert from "@lucide/svelte/icons/circle-alert";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import type { LiveState } from "../lib/activity.svelte";
  import { STATE_COLOR, STATE_LABEL } from "../lib/live";

  let {
    state,
    progress,
    size = 18,
  }: { state: LiveState; progress: number | null; size?: number } = $props();

  const RADIUS = 7;
  const CIRCUMFERENCE = 2 * Math.PI * RADIUS;
</script>

<span class="status" style:width="{size}px" style:height="{size}px" title={STATE_LABEL[state]}>
  {#if state === "running" && progress !== null}
    <svg viewBox="0 0 18 18" width={size} height={size} aria-hidden="true">
      <circle cx="9" cy="9" r={RADIUS} class="ring-track" />
      <circle
        cx="9"
        cy="9"
        r={RADIUS}
        class="ring-fill"
        stroke-dasharray={CIRCUMFERENCE}
        stroke-dashoffset={CIRCUMFERENCE * (1 - progress)}
      />
    </svg>
  {:else if state === "running"}
    <!-- Única animación continua de la isla: es un indicador de carga, que es justo
         el caso en el que una animación infinita comunica algo. -->
    <span class="spin"><LoaderCircle {size} strokeWidth={2.5} aria-hidden="true" /></span>
  {:else if state === "attention"}
    <CircleAlert {size} strokeWidth={2.5} color={STATE_COLOR.attention} aria-hidden="true" />
  {:else if state === "success"}
    <CircleCheck {size} strokeWidth={2.5} color={STATE_COLOR.success} aria-hidden="true" />
  {:else}
    <CircleX {size} strokeWidth={2.5} color={STATE_COLOR.error} aria-hidden="true" />
  {/if}
</span>

<style>
  .status {
    flex: none;
    display: grid;
    place-items: center;
  }

  circle {
    fill: none;
    stroke-width: 2.5;
  }

  .ring-track {
    stroke: var(--track);
  }

  .ring-fill {
    stroke: var(--text-primary);
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset 300ms ease-out;
  }

  .spin {
    display: grid;
    animation: spin 900ms linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation-duration: 2.4s;
    }
  }
</style>
