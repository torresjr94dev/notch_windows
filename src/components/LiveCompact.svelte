<script lang="ts">
  import type { LiveActivity } from "../lib/activity.svelte";
  import { STATE_COLOR, STATE_LABEL } from "../lib/live";
  import LiveGlyph from "./LiveGlyph.svelte";
  import LiveStatus from "./LiveStatus.svelte";

  let { live }: { live: LiveActivity } = $props();
</script>

<div class="compact" role="status" aria-label="{live.title}: {STATE_LABEL[live.state]}">
  <LiveGlyph icon={live.icon} size={16} color={STATE_COLOR[live.state]} />
  <span class="title">{live.title}</span>
  <LiveStatus state={live.state} progress={live.progress} />
</div>

<style>
  .compact {
    width: 100%;
    height: 100%;
    padding: 0 10px 0 12px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .title {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
