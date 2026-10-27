import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Lo emite el vigilante de Rust cuando el cursor entra o sale de la pastilla. */
const HOVER_EVENT = "island:hover";

/**
 * Traduce el hover crudo a "expandir/colapsar" con retardos. Sin el de entrada, cruzar
 * el mouse rumbo a una pestaña del navegador abriría la isla; sin el de salida, rozar
 * el borde la cerraría de golpe.
 *
 * Devuelve la función para dejar de escuchar.
 */
export function onHoverIntent(
  onChange: (expanded: boolean) => void,
  { enterMs = 120, leaveMs = 300 } = {},
): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let unlisten: UnlistenFn | undefined;
  let disposed = false;

  void listen<boolean>(HOVER_EVENT, ({ payload: hovered }) => {
    clearTimeout(timer);
    timer = setTimeout(() => onChange(hovered), hovered ? enterMs : leaveMs);
  }).then((fn) => {
    // `listen` es asíncrono: si ya nos desmontaron, soltar el listener en cuanto llegue.
    if (disposed) fn();
    else unlisten = fn;
  });

  return () => {
    disposed = true;
    clearTimeout(timer);
    unlisten?.();
  };
}
