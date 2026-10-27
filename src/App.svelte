<script lang="ts">
  import { fade } from "svelte/transition";
  import { Spring, prefersReducedMotion } from "svelte/motion";
  import Clock from "./lib/Clock.svelte";
  import { hitArea } from "./lib/hitArea";
  import { onHoverIntent } from "./lib/hoverIntent";
  import { reveal } from "./lib/reveal";

  type Mode = "compact" | "expanded";
  interface Shape {
    width: number;
    height: number;
    radius: number;
  }

  // En px CSS. La ventana mide 560×240 (tauri.conf.json): cabe la isla expandida más
  // el rebote del spring y su sombra.
  const SHAPES: Record<Mode, Shape> = {
    compact: { width: 190, height: 36, radius: 18 },
    expanded: { width: 400, height: 150, radius: 40 },
  };

  let mode = $state<Mode>("compact");

  // Rigidez alta con poco amortiguamiento: el rebote corto de la Dynamic Island. Si
  // Windows tiene "Efectos de animación" apagados, el cambio es instantáneo.
  const shape = new Spring(
    SHAPES.compact,
    prefersReducedMotion.current
      ? { stiffness: 1, damping: 1 }
      : { stiffness: 0.17, damping: 0.62 },
  );

  $effect(() => {
    shape.target = SHAPES[mode];
  });

  $effect(() => onHoverIntent((expanded) => (mode = expanded ? "expanded" : "compact")));

  if (!import.meta.env.DEV) {
    // El menú contextual de WebView2 (Atrás, Recargar, Inspeccionar) sobra en la isla.
    document.addEventListener("contextmenu", (e) => e.preventDefault());
  }
</script>

<main
  class="island"
  class:expanded={mode === "expanded"}
  style:width="{shape.current.width}px"
  style:height="{shape.current.height}px"
  style:border-radius="{shape.current.radius}px"
  {@attach hitArea}
>
  {#if mode === "expanded"}
    <div class="content" in:reveal out:fade={{ duration: 90 }}>
      <Clock />
    </div>
  {/if}
</main>

<style>
  .island {
    position: relative;
    overflow: hidden;
    margin: 8px auto 0;
    background: #000;
    transition: box-shadow 200ms ease;
  }

  .island.expanded {
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.35);
  }

  /* Tamaño fijo y centrado: mientras la pastilla crece, el contenido se "destapa"
     en vez de reacomodarse en cada cuadro. */
  .content {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 400px;
    height: 150px;
    margin: -75px 0 0 -200px;
    display: grid;
    place-items: center;
  }
</style>
