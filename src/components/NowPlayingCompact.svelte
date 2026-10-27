<script lang="ts">
  import Pause from "@lucide/svelte/icons/pause";
  import AudioLines from "@lucide/svelte/icons/audio-lines";
  import { island, type NowPlaying } from "../lib/activity.svelte";
  import Cover from "./Cover.svelte";
  import Visualizer from "./Visualizer.svelte";

  interface Props {
    track: NowPlaying;
    artworkUrl: string | null;
    accent: string;
  }

  let { track, artworkUrl, accent }: Props = $props();

  // Con el visualizador apagado (o mientras arranca la captura) Rust no manda niveles:
  // en vez de barras muertas, un ícono fijo dice "está sonando".
  const silent = $derived(island.spectrum.every((v) => v === 0));
</script>

<div class="compact">
  <Cover url={artworkUrl} size={22} radius={6} alt={track.album || track.title} />
  {#if !track.playing}
    <Pause size={15} strokeWidth={2.5} color="var(--text-secondary)" aria-label="En pausa" />
  {:else if silent}
    <AudioLines size={16} strokeWidth={2.25} color={accent} aria-label="Reproduciendo" />
  {:else}
    <Visualizer levels={island.spectrum} bars={5} height={14} color={accent} />
  {/if}
</div>

<style>
  .compact {
    width: 100%;
    height: 100%;
    padding: 0 9px 0 7px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
</style>
