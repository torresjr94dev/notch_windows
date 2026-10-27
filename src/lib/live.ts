import type { LiveState } from "./activity.svelte";

/** Texto del estado: acompaña al color y al ícono, nunca van solos. */
export const STATE_LABEL: Record<LiveState, string> = {
  running: "En curso",
  attention: "Necesita tu atención",
  success: "Listo",
  error: "Falló",
};

/** Token de color por estado (definidos en app.css). */
export const STATE_COLOR: Record<LiveState, string> = {
  running: "var(--text-primary)",
  attention: "var(--color-warning)",
  success: "var(--color-success)",
  error: "var(--color-danger)",
};

/** "40%" para progreso conocido; `null` si es indeterminado. */
export function progressLabel(progress: number | null): string | null {
  return progress === null ? null : `${Math.round(Math.min(1, Math.max(0, progress)) * 100)}%`;
}
