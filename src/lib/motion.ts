import type { Shape } from "./layout";

/** Rigidez del eje que lidera y del que lo sigue (springs de Svelte, 0–1). */
const LEAD = 0.22;
const FOLLOW = 0.13;
export const DAMPING = 0.62;

/**
 * Rigidez por eje para ir de una forma a otra. Como la Dynamic Island: al expandirse,
 * primero se abre a lo ancho y luego baja; al colapsar, primero sube y luego se cierra.
 * Un solo spring para todo hace que la pastilla "infle" como globo.
 */
export function axisStiffness(from: Shape, to: Shape): { width: number; height: number } {
  return to.height >= from.height
    ? { width: LEAD, height: FOLLOW }
    : { width: FOLLOW, height: LEAD };
}

/** Radio visible: nunca más que medio alto, aunque su spring vaya adelantado. */
export function visibleRadius(radius: number, height: number): number {
  return Math.max(0, Math.min(radius, height / 2));
}
