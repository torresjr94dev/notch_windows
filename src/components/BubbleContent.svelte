<script lang="ts">
  import type { Activity } from "../lib/activity.svelte";
  import { STATE_LABEL } from "../lib/live";
  import Cover from "./Cover.svelte";
  import LiveStatus from "./LiveStatus.svelte";

  interface Props {
    activity: Activity;
    artworkUrl: string | null;
  }

  let { activity, artworkUrl }: Props = $props();
</script>

<!-- La burbuja del split resume la actividad secundaria en un solo glifo. -->
<div class="content">
  {#if activity.data.kind === "nowPlaying"}
    <Cover url={artworkUrl} size={24} radius={12} alt={activity.data.title} />
  {:else if activity.data.kind === "live"}
    <span role="status" aria-label="{activity.data.title}: {STATE_LABEL[activity.data.state]}">
      <LiveStatus state={activity.data.state} progress={activity.data.progress} size={18} />
    </span>
  {/if}
</div>

<style>
  .content {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }
</style>
