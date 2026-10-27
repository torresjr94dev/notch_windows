import { cubicOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";

/**
 * Entrada estilo Dynamic Island: el contenido aparece desenfocado y un poco chico, y se
 * asienta mientras la pastilla termina de crecer. El retardo evita que se vea apachurrado
 * en los primeros cuadros del spring.
 */
export function reveal(_node: Element, { delay = 70, duration = 260 } = {}): TransitionConfig {
  return {
    delay,
    duration,
    easing: cubicOut,
    css: (t) =>
      `opacity: ${t}; transform: scale(${0.92 + 0.08 * t}); filter: blur(${(1 - t) * 6}px);`,
  };
}
