<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { fade, scale } from "svelte/transition";
  import { Spring, prefersReducedMotion } from "svelte/motion";
  import BatteryNotice from "./components/BatteryNotice.svelte";
  import BubbleContent from "./components/BubbleContent.svelte";
  import CapsLockNotice from "./components/CapsLockNotice.svelte";
  import Clock from "./components/Clock.svelte";
  import LevelNotice from "./components/LevelNotice.svelte";
  import LiveCompact from "./components/LiveCompact.svelte";
  import LiveExpanded from "./components/LiveExpanded.svelte";
  import NowPlayingCompact from "./components/NowPlayingCompact.svelte";
  import NowPlayingExpanded from "./components/NowPlayingExpanded.svelte";
  import PrivacyBubble from "./components/PrivacyBubble.svelte";
  import { connect, island, type Activity, type NowPlaying } from "./lib/activity.svelte";
  import { loadArtwork, type Artwork } from "./lib/artwork";
  import { createAccumulator, wheelPixels } from "./lib/gestures";
  import { hitArea } from "./lib/hitArea";
  import { onHoverIntent } from "./lib/hoverIntent";
  import { SHAPES, layout } from "./lib/layout";
  import { DAMPING, axisStiffness, visibleRadius } from "./lib/motion";
  import { focusTarget } from "./lib/live";
  import { reveal } from "./lib/reveal";

  /** Margen sobre la pastilla, en px CSS. */
  const TOP = 8;
  /** Burbuja del split: diámetro y separación de la pastilla. */
  const BUBBLE = 36;
  const GAP = 8;
  /** Cuánto mueve el volumen una muesca de la rueda. */
  const VOLUME_STEP = 0.02;

  let hovered = $state(false);
  // Un clic expande sin esperar el retardo del hover (no depender solo del hover). Se
  // olvida en cuanto el cursor sale.
  let clicked = $state(false);
  let pressed = $state(false);

  const current = $derived(layout(island.view, hovered || clicked));
  const target = $derived(SHAPES[current.face][current.mode]);

  // Si Windows tiene "Efectos de animación" apagados, todo es instantáneo.
  const reduced = prefersReducedMotion.current;
  const spring = <T,>(value: T, stiffness: number) =>
    new Spring(value, reduced ? { stiffness: 1, damping: 1 } : { stiffness, damping: DAMPING });

  // Un spring por eje: al expandir se abre primero a lo ancho y luego baja (ver motion.ts).
  const width = spring(SHAPES.idle.compact.width, 0.17);
  const height = spring(SHAPES.idle.compact.height, 0.17);
  const radius = spring(SHAPES.idle.compact.radius, 0.17);
  let previous = SHAPES.idle.compact;
  $effect(() => {
    const next = target;
    if (!reduced) {
      const stiffness = axisStiffness(previous, next);
      width.stiffness = stiffness.width;
      height.stiffness = stiffness.height;
      radius.stiffness = stiffness.height;
    }
    previous = next;
    width.target = next.width;
    height.target = next.height;
    radius.target = next.radius;
  });
  const pill = $derived({
    w: width.current,
    h: height.current,
    r: visibleRadius(radius.current, height.current),
  });

  $effect(() =>
    onHoverIntent((value) => {
      hovered = value;
      if (!value) clicked = false;
    }),
  );

  $effect(() => connect());

  const nowPlaying = (activity: Activity | null): NowPlaying | null =>
    activity?.data.kind === "nowPlaying" ? activity.data : null;

  /** La canción que se dibuja en la pastilla ahora mismo. */
  const track = $derived(nowPlaying(current.activity));
  /** La canción esté donde esté (también en la burbuja mientras dura un aviso): así la
   *  carátula no se descarta y vuelve a cargar cada vez que sube el volumen. */
  const music = $derived(nowPlaying(island.view.primary) ?? nowPlaying(island.view.secondary));

  // Split: la secundaria sale de la pastilla como una gota (filtro gooey) y se separa.
  // Expandida, la pastilla manda y la burbuja se vuelve a meter.
  const secondary = $derived(island.view.secondary);
  const showBubble = $derived(secondary !== null && current.mode === "compact");
  /** Distancia de la burbuja al borde derecho de la pastilla; -BUBBLE = escondida detrás. */
  const bubbleGap = spring(-BUBBLE, 0.14);
  $effect(() => {
    bubbleGap.target = showBubble ? GAP : -BUBBLE;
  });
  const bubbleMounted = $derived(bubbleGap.current > -BUBBLE + 0.5);
  // La última secundaria se conserva para dibujar la burbuja mientras se esconde.
  let lastSecondary = $state<Activity | null>(null);
  $effect(() => {
    if (secondary) lastSecondary = secondary;
  });
  const bubbleTop = $derived(TOP + (Math.min(pill.h, 38) - BUBBLE) / 2);

  const privacy = $derived(island.view.privacy);
  const showPrivacy = $derived(privacy.microphone.length > 0 || privacy.camera.length > 0);

  let artwork = $state<Artwork | null>(null);
  $effect(() => {
    const id = music?.artworkId;
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
  // Algo externo necesita al usuario: anillo ámbar fijo (sin parpadeo; el color, el
  // ícono y el texto ya lo dicen).
  const attention = $derived(
    current.activity?.data.kind === "live" && current.activity.data.state === "attention",
  );

  // Gestos. Siempre hay alternativa visible: los botones de Now Playing y las teclas de
  // volumen del teclado.
  /** Empujón lateral al cambiar de canción: confirma que el swipe se registró. */
  const nudge = spring(0, 0.2);
  const changeVolume = createAccumulator({
    threshold: 100,
    onStep: (direction) => void invoke("adjust_volume", { delta: -direction * VOLUME_STEP }),
  });
  const changeTrack = createAccumulator({
    threshold: 160,
    cooldownMs: 700,
    onStep: (direction) => {
      void invoke("media_control", { command: { action: direction > 0 ? "next" : "previous" } });
      if (!reduced) {
        nudge.set(-direction * 10, { instant: true });
        nudge.target = 0;
      }
    },
  });

  function onWheel(event: WheelEvent) {
    const dx = wheelPixels(event.deltaX, event.deltaMode);
    const dy = wheelPixels(event.deltaY, event.deltaMode);
    // Horizontal (swipe de touchpad o Shift + rueda) = canciones; vertical = volumen.
    if (Math.abs(dx) > Math.abs(dy)) {
      if (music) changeTrack(dx, event.timeStamp);
    } else {
      changeVolume(dy, event.timeStamp);
    }
  }

  /** Clic en la pastilla o la burbuja: si la actividad viene de una ventana (Claude en
   *  su terminal), te lleva ahí; si no, expande la isla sin esperar el hover. */
  function activate(activity: Activity | null) {
    const id = focusTarget(activity);
    if (id) void invoke("focus_activity", { id });
    else clicked = true;
  }

  function onPress(event: PointerEvent) {
    // Los botones tienen su propio feedback; la pastilla entera no debe encogerse.
    pressed = !(event.target as Element).closest("button");
  }

  if (!import.meta.env.DEV) {
    // El menú contextual de WebView2 (Atrás, Recargar, Inspeccionar) sobra en la isla.
    document.addEventListener("contextmenu", (e) => e.preventDefault());
  }
</script>

<!-- Gooey: blur + umbral de alfa. Las formas que se tocan se funden; `atop` devuelve los
     bordes nítidos donde ya había forma. -->
<svg class="defs" aria-hidden="true">
  <filter id="goo">
    <feGaussianBlur in="SourceGraphic" stdDeviation="5" result="blur" />
    <feColorMatrix
      in="blur"
      mode="matrix"
      values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7"
      result="goo"
    />
    <feComposite in="SourceGraphic" in2="goo" operator="atop" />
  </filter>
</svg>

<!-- La ventana no puede recibir foco de teclado (focusable: false en Rust): clic, rueda y
     presión son atajos de mouse; los controles reales son <button>. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="stage"
  style:--accent={accentRgb}
  style:transform="translateX({nudge.current}px) scale({pressed ? 0.97 : 1})"
  style:transform-origin="50% {TOP + pill.h / 2}px"
  onwheel={onWheel}
  onpointerdown={onPress}
  onpointerup={() => (pressed = false)}
  onpointerleave={() => (pressed = false)}
>
  <!-- Capa de formas: el negro de la pastilla y la burbuja, con goo y glow. Va separada
       del contenido porque el blur del goo emborronaría el texto. -->
  <div
    class="shapes"
    class:goo={bubbleMounted}
    class:expanded={current.mode === "expanded"}
    class:glowing
    class:attention
  >
    <div
      class="shape"
      style:left="calc(50% - {pill.w / 2}px)"
      style:top="{TOP}px"
      style:width="{pill.w}px"
      style:height="{pill.h}px"
      style:border-radius="{pill.r}px"
    ></div>
    {#if bubbleMounted}
      <div
        class="shape"
        style:left="calc(50% + {pill.w / 2 + bubbleGap.current}px)"
        style:top="{bubbleTop}px"
        style:width="{BUBBLE}px"
        style:height="{BUBBLE}px"
        style:border-radius="50%"
      ></div>
    {/if}
  </div>

  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <main
    class="pill"
    style:left="calc(50% - {pill.w / 2}px)"
    style:top="{TOP}px"
    style:width="{pill.w}px"
    style:height="{pill.h}px"
    style:border-radius="{pill.r}px"
    class:focusable={focusTarget(current.activity) !== null}
    onclick={() => activate(current.activity)}
  >
    {#key `${current.face}:${current.mode}`}
      <!-- Tamaño fijo y centrado: mientras la pastilla crece, el contenido se "destapa"
           en vez de reacomodarse en cada cuadro. -->
      <div
        class="content"
        style:width="{target.width}px"
        style:height="{target.height}px"
        style:margin="{-target.height / 2}px 0 0 {-target.width / 2}px"
        in:reveal
        out:fade={{ duration: 90 }}
      >
        {#if current.activity?.data.kind === "volume"}
          <LevelNotice
            kind="volume"
            level={current.activity.data.level}
            muted={current.activity.data.muted}
          />
        {:else if current.activity?.data.kind === "brightness"}
          <LevelNotice kind="brightness" level={current.activity.data.level} />
        {:else if current.activity?.data.kind === "battery"}
          <BatteryNotice {...current.activity.data} />
        {:else if current.activity?.data.kind === "capsLock"}
          <CapsLockNotice on={current.activity.data.on} />
        {:else if current.activity?.data.kind === "live" && current.mode === "compact"}
          <LiveCompact live={current.activity.data} />
        {:else if current.activity?.data.kind === "live"}
          <LiveExpanded live={current.activity.data} onfocus={() => activate(current.activity)} />
        {:else if track && current.mode === "compact"}
          <NowPlayingCompact {track} artworkUrl={artwork?.url ?? null} {accent} />
        {:else if track}
          <NowPlayingExpanded {track} artworkUrl={artwork?.url ?? null} {accent} />
        {:else if current.mode === "expanded"}
          <div class="center"><Clock /></div>
        {/if}
      </div>
    {/key}
  </main>

  {#if bubbleMounted && lastSecondary}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="bubble"
      style:left="calc(50% + {pill.w / 2 + bubbleGap.current}px)"
      style:top="{bubbleTop}px"
      style:width="{BUBBLE}px"
      style:height="{BUBBLE}px"
      class:focusable={focusTarget(lastSecondary) !== null}
      onclick={() => activate(lastSecondary)}
    >
      {#if showBubble}
        <div class="bubble-content" transition:fade={{ duration: 120 }}>
          <BubbleContent activity={lastSecondary} artworkUrl={artwork?.url ?? null} />
        </div>
      {/if}
    </div>
  {/if}

  <!-- Área sensible para Rust: la pastilla más la burbuja (ver hitArea.ts). No recibe
       eventos; solo se mide. -->
  <div
    class="hit"
    style:left="calc(50% - {pill.w / 2}px)"
    style:top="{TOP}px"
    style:width="{pill.w + (showBubble ? GAP + BUBBLE : 0)}px"
    style:height="{pill.h}px"
    {@attach hitArea}
  ></div>
</div>

{#if showPrivacy}
  <!-- A la izquierda de la pastilla y a la altura de la forma compacta, siguiendo el
       spring cuadro por cuadro. Es solo indicador: queda fuera del área de clic. -->
  <div
    class="privacy"
    style:top="{TOP + (Math.min(pill.h, 38) - 26) / 2}px"
    style:right="calc(50% + {pill.w / 2 + 8}px)"
    transition:scale={{ duration: 200, start: 0.6 }}
  >
    <PrivacyBubble {privacy} />
  </div>
{/if}

<style>
  .defs {
    position: absolute;
    width: 0;
    height: 0;
  }

  .stage {
    position: fixed;
    inset: 0;
    transition: transform 120ms ease-out;
  }

  .shapes {
    position: absolute;
    inset: 0;
    /* Goo y glow se combinan en un solo `filter`; cada clase llena su parte. */
    --goo-filter: ;
    --glow-filter: ;
    filter: var(--goo-filter) var(--glow-filter);
    pointer-events: none;
  }

  .shapes.goo {
    --goo-filter: url(#goo);
  }

  .shapes.expanded {
    --glow-filter: drop-shadow(0 10px 22px rgb(0 0 0 / 0.35));
  }

  /* El glow toma el color de la carátula: la isla "suena" con la canción. */
  .shapes.glowing {
    --glow-filter: drop-shadow(0 0 1px rgb(var(--accent) / 0.6))
      drop-shadow(0 6px 18px rgb(var(--accent) / 0.45));
  }

  .shapes.attention {
    --glow-filter: drop-shadow(0 0 1.5px rgb(255 159 10 / 0.85))
      drop-shadow(0 6px 18px rgb(255 159 10 / 0.45));
  }

  .shape {
    position: absolute;
    background: var(--island-bg);
  }

  .pill {
    position: absolute;
    overflow: hidden;
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

  .bubble {
    position: absolute;
  }

  /* Lleva a otra ventana: el cursor lo anuncia antes del clic. */
  .pill.focusable,
  .bubble.focusable {
    cursor: pointer;
  }

  .bubble-content {
    width: 100%;
    height: 100%;
  }

  .hit {
    position: absolute;
    pointer-events: none;
  }

  .privacy {
    position: fixed;
  }
</style>
