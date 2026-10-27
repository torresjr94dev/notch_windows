<script lang="ts">
  import { fade } from "svelte/transition";
  import { Spring, prefersReducedMotion } from "svelte/motion";
  import Clock from "./components/Clock.svelte";
  import NowPlayingCompact from "./components/NowPlayingCompact.svelte";
  import NowPlayingExpanded from "./components/NowPlayingExpanded.svelte";
  import { connect, island } from "./lib/activity.svelte";
  import { loadArtwork, type Artwork } from "./lib/artwork";
  import { hitArea } from "./lib/hitArea";
  import { onHoverIntent } from "./lib/hoverIntent";
  import { SHAPES, layout } from "./lib/layout";
  import { reveal } from "./lib/reveal";

  let hovered = $state(false);
  // Un clic expande sin esperar el retardo del hover (no depender solo del hover). Se
  // olvida en cuanto el cursor sale.
  let clicked = $state(false);

  const current = $derived(layout(island.view, hovered || clicked));
  const shapeTarget = $derived(SHAPES[current.face][current.mode]);

  // Rigidez alta con poco amortiguamiento: el rebote corto de la Dynamic Island. Si
  // Windows tiene "Efectos de animación" apagados, el cambio es instantáneo.
  const shape = new Spring(
    SHAPES.idle.compact,
    prefersReducedMotion.current
      ? { stiffness: 1, damping: 1 }
      : { stiffness: 0.17, damping: 0.62 },
  );

  $effect(() => {
    shape.target = shapeTarget;
  });

  $effect(() =>
    onHoverIntent((value) => {
      hovered = value;
      if (!value) clicked = false;
    }),
  );

  $effect(() => connect());

  const track = $derived(
    current.activity?.data.kind === "nowPlaying" ? current.activity.data : null,
  );

  let artwork = $state<Artwork | null>(null);
  $effect(() => {
    const id = track?.artworkId;
    if (!id) {
      artwork = null;
      return;
    }
    let stale = false;
    void loadArtwork(id).then((loaded) => {
      if (!stale) artwork = loaded;
    });
    return () => {
      stale = true;
    };
  });

  const accentRgb = $derived(artwork?.accent?.join(" ") ?? "255 255 255");
  const accent = $derived(`rgb(${accentRgb})`);
  const glowing = $derived(track?.playing === true && artwork?.accent != null);

  if (!import.meta.env.DEV) {
    // El menú contextual de WebView2 (Atrás, Recargar, Inspeccionar) sobra en la isla.
    document.addEventListener("contextmenu", (e) => e.preventDefault());
  }
</script>

<!-- La ventana no puede recibir foco de teclado (focusable: false en Rust): el clic es
     un atajo de mouse para expandir; los controles reales son <button>. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<main
  class="island"
  class:expanded={current.mode === "expanded"}
  class:glowing
  style:width="{shape.current.width}px"
  style:height="{shape.current.height}px"
  style:border-radius="{shape.current.radius}px"
  style:--accent={accentRgb}
  onclick={() => (clicked = true)}
  {@attach hitArea}
>
  {#key `${current.face}:${current.mode}`}
    <!-- Tamaño fijo y centrado: mientras la pastilla crece, el contenido se "destapa"
         en vez de reacomodarse en cada cuadro. -->
    <div
      class="content"
      style:width="{shapeTarget.width}px"
      style:height="{shapeTarget.height}px"
      style:margin="{-shapeTarget.height / 2}px 0 0 {-shapeTarget.width / 2}px"
      in:reveal
      out:fade={{ duration: 90 }}
    >
      {#if track && current.mode === "compact"}
        <NowPlayingCompact {track} artworkUrl={artwork?.url ?? null} {accent} />
      {:else if track}
        <NowPlayingExpanded {track} artworkUrl={artwork?.url ?? null} {accent} />
      {:else if current.mode === "expanded"}
        <div class="center"><Clock /></div>
      {/if}
    </div>
  {/key}
</main>

<style>
  .island {
    position: relative;
    overflow: hidden;
    margin: 8px auto 0;
    background: var(--island-bg);
    transition: box-shadow 300ms ease;
  }

  .island.expanded {
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.35);
  }

  /* El glow toma el color de la carátula: la isla "suena" con la canción. */
  .island.glowing {
    box-shadow:
      0 0 0 1px rgb(var(--accent) / 0.2),
      0 6px 26px -6px rgb(var(--accent) / 0.55);
  }

  .content {
    position: absolute;
    left: 50%;
    top: 50%;
  }

  .center {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }
</style>
