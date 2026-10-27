<script lang="ts">
  import Mic from "@lucide/svelte/icons/mic";
  import Video from "@lucide/svelte/icons/video";
  import type { Privacy } from "../lib/activity.svelte";
  import { privacyLabel } from "../lib/privacy";

  let { privacy }: { privacy: Privacy } = $props();
</script>

<!-- Como los puntos de privacidad del iPhone: naranja = micrófono, verde = cámara.
     El ícono y el aria-label dicen lo mismo que el color. -->
<div class="bubble" role="status" aria-label={privacyLabel(privacy)}>
  {#if privacy.microphone.length}
    <Mic size={13} strokeWidth={2.5} color="var(--privacy-mic)" aria-hidden="true" />
  {/if}
  {#if privacy.camera.length}
    <Video size={13} strokeWidth={2.5} color="var(--privacy-camera)" aria-hidden="true" />
  {/if}
</div>

<style>
  .bubble {
    height: 26px;
    min-width: 26px;
    padding: 0 6.5px;
    box-sizing: border-box;
    border-radius: 13px;
    background: var(--island-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
  }
</style>
