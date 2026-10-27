<script lang="ts">
  import ArrowUpRight from "@lucide/svelte/icons/arrow-up-right";
  import type { LiveActivity } from "../lib/activity.svelte";
  import { STATE_COLOR, STATE_LABEL, progressLabel } from "../lib/live";
  import LiveGlyph from "./LiveGlyph.svelte";
  import LiveStatus from "./LiveStatus.svelte";

  interface Props {
    live: LiveActivity;
    /** Lleva al frente la ventana de origen; solo si `live.focusable`. */
    onfocus?: () => void;
  }

  let { live, onfocus }: Props = $props();

  const percent = $derived(progressLabel(live.progress));
</script>

<div class="expanded">
  <div class="header">
    <span class="badge" style:--tone={STATE_COLOR[live.state]}>
      <LiveGlyph icon={live.icon} size={22} color={STATE_COLOR[live.state]} />
    </span>
    <div class="meta">
      <p class="title" title={live.title}>{live.title}</p>
      {#if live.subtitle}
        <p class="subtitle" title={live.subtitle}>{live.subtitle}</p>
      {/if}
    </div>
    <LiveStatus state={live.state} progress={live.progress} size={22} />
  </div>

  <div class="footer">
    {#if percent !== null && live.state === "running"}
      <span class="track"><span class="fill" style:transform="scaleX({live.progress})"></span></span
      >
      <span class="value">{percent}</span>
    {:else}
      <span class="state" style:color={STATE_COLOR[live.state]}>{STATE_LABEL[live.state]}</span>
    {/if}
    {#if live.focusable && onfocus}
      <!-- Toda la isla es clickeable, pero el botón hace visible que lo es. -->
      <button
        class="go"
        onclick={(event) => {
          event.stopPropagation();
          onfocus();
        }}
      >
        Ir a la ventana <ArrowUpRight size={14} strokeWidth={2.5} aria-hidden="true" />
      </button>
    {/if}
  </div>
</div>

<style>
  .expanded {
    width: 100%;
    height: 100%;
    padding: 18px 20px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  /* Fondo del mismo tono que el estado, muy tenue. */
  .badge {
    flex: none;
    width: 44px;
    height: 44px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--tone) 16%, transparent);
  }

  .meta {
    flex: 1;
    min-width: 0;
  }

  .title,
  .subtitle {
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
  }

  .subtitle {
    font-size: 13px;
    line-height: 1.35;
    color: var(--text-secondary);
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 16px;
  }

  .track {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--text-primary);
    transform-origin: left;
    transition: transform 300ms ease-out;
  }

  .go {
    margin-left: auto;
    min-height: 28px;
    padding: 0 10px 0 12px;
    border: 0;
    border-radius: 14px;
    background: var(--surface-raised);
    color: var(--text-primary);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    transition:
      background-color 150ms ease,
      transform 120ms ease;
  }

  .go:hover {
    background: var(--surface-hover);
  }

  .go:active {
    transform: scale(0.95);
  }

  .value,
  .state {
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
