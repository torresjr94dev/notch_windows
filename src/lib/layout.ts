import type { Activity, IslandView } from "./activity.svelte";

export type Mode = "compact" | "expanded";
/** Qué dibuja la pastilla. "idle" = sin actividades (compacta vacía, expandida con reloj). */
export type Face = "idle" | Activity["data"]["kind"];

export interface Shape {
  width: number;
  height: number;
  radius: number;
}

/** Volumen, brillo y batería comparten forma: un aviso ancho y bajo. */
const NOTICE: Shape = { width: 300, height: 38, radius: 19 };

/**
 * Tamaños en px CSS. La ventana mide 560×240 (`island::WINDOW_SIZE` en Rust): la forma
 * más grande más el rebote del spring y el glow tienen que caber.
 */
export const SHAPES: Record<Face, Record<Mode, Shape>> = {
  idle: {
    compact: { width: 190, height: 36, radius: 18 },
    expanded: { width: 400, height: 150, radius: 40 },
  },
  nowPlaying: {
    compact: { width: 250, height: 36, radius: 18 },
    expanded: { width: 420, height: 176, radius: 42 },
  },
  // Los avisos del sistema son transitorios: nunca se expanden.
  volume: { compact: NOTICE, expanded: NOTICE },
  brightness: { compact: NOTICE, expanded: NOTICE },
  battery: { compact: NOTICE, expanded: NOTICE },
  capsLock: {
    compact: { width: 260, height: 38, radius: 19 },
    expanded: { width: 260, height: 38, radius: 19 },
  },
};

export interface Layout {
  face: Face;
  mode: Mode;
  activity: Activity | null;
}

/**
 * Qué mostrar según la vista de Rust y si el usuario está sobre la isla.
 *
 * - Un transitorio (volumen, brillo…) dura un par de segundos y gana aunque haya hover.
 * - Con hover se expande la actividad principal; sin actividades, el reloj.
 */
export function layout(view: IslandView, expanded: boolean): Layout {
  const primary = view.primary;
  if (primary?.transient) {
    return { face: primary.data.kind, mode: "compact", activity: primary };
  }
  return {
    face: primary?.data.kind ?? "idle",
    mode: expanded ? "expanded" : "compact",
    activity: primary,
  };
}
