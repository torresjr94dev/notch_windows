<script lang="ts">
  import Music from "@lucide/svelte/icons/music";

  interface Props {
    url: string | null | undefined;
    size: number;
    radius: number;
    alt?: string;
  }

  let { url, size, radius, alt = "" }: Props = $props();
</script>

<div class="cover" style:width="{size}px" style:height="{size}px" style:border-radius="{radius}px">
  {#if url}
    <img src={url} {alt} width={size} height={size} />
  {:else}
    <Music size={size * 0.5} strokeWidth={2} aria-hidden="true" />
  {/if}
</div>

<style>
  .cover {
    flex: none;
    display: grid;
    place-items: center;
    overflow: hidden;
    background: var(--surface-raised);
    color: var(--text-secondary);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
