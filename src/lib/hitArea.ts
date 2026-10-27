import { invoke } from "@tauri-apps/api/core";
import type { Attachment } from "svelte/attachments";

/** Rectángulo en píxeles físicos relativo a la ventana, como lo espera `set_hit_rect`. */
export interface HitRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Convierte la caja de la pastilla (px CSS) al área sensible en píxeles físicos.
 *
 * El área se estira hasta el borde superior de la ventana: el usuario avienta el mouse
 * contra el tope de la pantalla (ley de Fitts) y ahí debe atraparlo la isla, aunque la
 * pastilla tenga margen arriba.
 */
export function toHitRect(box: DOMRectReadOnly, dpr: number): HitRect {
  const left = Math.floor(box.left * dpr);
  const right = Math.ceil(box.right * dpr);
  return { x: left, y: 0, width: right - left, height: Math.ceil(box.bottom * dpr) };
}

/** Mantiene a Rust al tanto del área sensible del elemento. */
export const hitArea: Attachment<HTMLElement> = (node) => {
  let frame = 0;
  let last = "";

  const report = () => {
    frame = 0;
    const rect = toHitRect(node.getBoundingClientRect(), window.devicePixelRatio);
    const key = `${rect.x},${rect.width},${rect.height}`;
    if (key === last) return;
    last = key;
    void invoke("set_hit_rect", { rect });
  };
  // Un reporte por cuadro como máximo: durante un spring el tamaño cambia en cada frame.
  const schedule = () => {
    frame ||= requestAnimationFrame(report);
  };

  const resize = new ResizeObserver(schedule);
  resize.observe(node);

  // Cambiar de escala (DPI) no cambia el tamaño CSS y el ResizeObserver no se entera,
  // así que devicePixelRatio se vigila aparte.
  let dprQuery: MediaQueryList;
  const onDprChange = () => {
    schedule();
    watchDpr();
  };
  const watchDpr = () => {
    dprQuery = matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
    dprQuery.addEventListener("change", onDprChange, { once: true });
  };
  watchDpr();

  return () => {
    resize.disconnect();
    dprQuery.removeEventListener("change", onDprChange);
    cancelAnimationFrame(frame);
  };
};
