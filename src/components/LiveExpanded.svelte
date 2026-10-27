<script lang="ts">
  import type { LiveActivity } from "../lib/activity.svelte";
  import { STATE_COLOR, STATE_LABEL, progressLabel } from "../lib/live";
  import LiveGlyph from "./LiveGlyph.svelte";
  import LiveStatus from "./LiveStatus.svelte";

  let { live }: { live: LiveActivity } = $props();

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

  .value,
  .state {
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
</style>
