import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Espejo de `src-tauri/src/activity/mod.rs`. Mantener en sync.

export interface NowPlaying {
  title: string;
  artist: string;
  album: string;
  app: string;
  artworkId: string | null;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  /** Cuándo (ms epoch) se midió `positionMs`; la posición actual se interpola. */
  sampledAtMs: number;
}

export type ActivityData = { kind: "nowPlaying" } & NowPlaying;

export interface Activity {
  id: string;
  priority: number;
  transient: boolean;
  data: ActivityData;
}

export interface IslandView {
  primary: Activity | null;
  secondary: Activity | null;
}

export const BANDS = 16;

/** Estado compartido por todos los componentes de esta isla. */
export const island = $state({
  view: { primary: null, secondary: null } as IslandView,
  spectrum: new Array<number>(BANDS).fill(0),
});

/**
 * Conecta la isla con Rust. Estos eventos son para todas las islas, así que aquí sí va
 * el `listen` global (a diferencia del hover, que es por ventana).
 */
export function connect(): () => void {
  let receivedEvent = false;
  const unlisteners = [
    listen<IslandView>("island:view", ({ payload }) => {
      receivedEvent = true;
      island.view = payload;
    }),
    listen<number[]>("island:spectrum", ({ payload }) => {
      island.spectrum = payload;
    }),
  ];
  // Una isla que nace después (monitor nuevo) no vio los eventos anteriores. Si llega un
  // evento antes que esta respuesta, el evento es más nuevo y gana.
  void invoke<IslandView>("get_view").then((view) => {
    if (!receivedEvent) island.view = view;
  });

  return () => {
    for (const unlisten of unlisteners) void unlisten.then((fn) => fn());
  };
}
