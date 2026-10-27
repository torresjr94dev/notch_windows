<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import SkipBack from "@lucide/svelte/icons/skip-back";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import { island, type NowPlaying } from "../lib/activity.svelte";
  import { currentPositionMs, formatTime } from "../lib/time";
  import Cover from "./Cover.svelte";
  import Visualizer from "./Visualizer.svelte";

  interface Props {
    track: NowPlaying;
    artworkUrl: string | null;
    accent: string;
  }

  let { track, artworkUrl, accent }: Props = $props();

  type Command =
    { action: "togglePlayPause" | "next" | "previous" } | { action: "seek"; positionMs: number };
  const control = (command: Command) => void invoke("media_control", { command });

  // La posición se interpola localmente; solo hace falta un reloj mientras suena.
  let now = $state(Date.now());
  $effect(() => {
    if (!track.playing) return;
    now = Date.now();
    const id = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(id);
  });

  const position = $derived(currentPositionMs(track, now));
  const ratio = $derived(track.durationMs > 0 ? position / track.durationMs : 0);
  const subtitle = $derived([track.artist, track.app].filter(Boolean).join(" · "));

  function seek(event: MouseEvent) {
    const bar = event.currentTarget as HTMLElement;
    const { left, width } = bar.getBoundingClientRect();
    const fraction = Math.min(1, Math.max(0, (event.clientX - left) / width));
    control({ action: "seek", positionMs: Math.round(fraction * track.durationMs) });
  }
</script>

<div class="expanded">
  <div class="header">
    <Cover url={artworkUrl} size={56} radius={14} alt={track.album || track.title} />
    <div class="meta">
      <p class="title" title={track.title}>{track.title}</p>
      <p class="subtitle" title={subtitle}>{subtitle}</p>
    </div>
    <Visualizer
      levels={island.spectrum}
      bars={8}
      height={22}
      color={accent}
      active={track.playing}
    />
  </div>

  {#if track.durationMs > 0}
    <div class="timeline">
      <span class="time">{formatTime(position)}</span>
      <button class="track" onclick={seek} aria-label="Cambiar posición">
        <span class="fill" style:transform="scaleX({ratio})" style:background={accent}></span>
      </button>
      <span class="time">-{formatTime(track.durationMs - position)}</span>
    </div>
  {/if}

  <div class="controls">
    <button class="control" aria-label="Anterior" onclick={() => control({ action: "previous" })}>
      <SkipBack size={22} fill="currentColor" strokeWidth={1.5} />
    </button>
    <button
      class="control primary"
      aria-label={track.playing ? "Pausar" : "Reproducir"}
      onclick={() => control({ action: "togglePlayPause" })}
    >
      {#if track.playing}
        <Pause size={28} fill="currentColor" strokeWidth={1.5} />
      {:else}
        <Play size={28} fill="currentColor" strokeWidth={1.5} />
      {/if}
    </button>
    <button class="control" aria-label="Siguiente" onclick={() => control({ action: "next" })}>
      <SkipForward size={22} fill="currentColor" strokeWidth={1.5} />
    </button>
  </div>
</div>

<style>
  .expanded {
    width: 100%;
    height: 100%;
    padding: 18px 22px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
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

  .timeline {
    margin-top: 14px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .time {
    width: 40px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }

  .time:last-child {
    text-align: right;
  }

  /* El área clickeable es más alta que la barra visible: atinarle a 4 px es imposible. */
  .track {
    flex: 1;
    height: 16px;
    padding: 6px 0;
    border: 0;
    background: none;
    cursor: pointer;
    display: block;
  }

  .track::before,
  .fill {
    display: block;
    height: 4px;
    border-radius: 2px;
  }

  .track::before {
    content: "";
    background: var(--track);
  }

  .fill {
    margin-top: -4px;
    transform-origin: left;
  }

  .controls {
    margin-top: auto;
    display: flex;
    justify-content: center;
    gap: 28px;
  }

  .control {
    width: 44px;
    height: 44px;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--text-primary);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition:
      background-color 150ms ease,
      transform 120ms ease;
  }

  .control:hover {
    background: var(--surface-hover);
  }

  .control:active {
    transform: scale(0.9);
  }
</style>
