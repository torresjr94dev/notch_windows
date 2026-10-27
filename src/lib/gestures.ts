export interface AccumulatorOptions {
  /** Unidades de delta por paso. Una muesca de mouse son ~100 px. */
  threshold: number;
  /** Tras un paso, ignora deltas este tiempo: la inercia de un swipe de touchpad no debe
   *  saltar tres canciones de golpe. */
  cooldownMs?: number;
  /** Si pasa este tiempo sin deltas, lo acumulado se descarta: medio giro de hace un
   *  minuto no debe sumarse al de ahora. */
  idleResetMs?: number;
  onStep: (direction: 1 | -1) => void;
}

/**
 * Convierte deltas de rueda o touchpad (continuos y de tamaño variable) en pasos
 * discretos. Devuelve la función que recibe cada delta con su timestamp.
 */
export function createAccumulator({
  threshold,
  cooldownMs = 0,
  idleResetMs = 400,
  onStep,
}: AccumulatorOptions): (delta: number, timeMs: number) => void {
  let total = 0;
  let lastInput = -Infinity;
  let lastStep = -Infinity;

  return (delta, timeMs) => {
    if (timeMs - lastInput > idleResetMs) total = 0;
    lastInput = timeMs;
    if (timeMs - lastStep < cooldownMs) return;

    total += delta;
    while (Math.abs(total) >= threshold) {
      const direction = total > 0 ? 1 : -1;
      onStep(direction);
      total -= direction * threshold;
      if (cooldownMs > 0) {
        lastStep = timeMs;
        total = 0;
        break;
      }
    }
  };
}

/** `WheelEvent.deltaMode`: 0 = píxeles, 1 = líneas, 2 = páginas. Todo a píxeles. */
export function wheelPixels(delta: number, deltaMode: number): number {
  return deltaMode === 1 ? delta * 40 : deltaMode === 2 ? delta * 800 : delta;
}
