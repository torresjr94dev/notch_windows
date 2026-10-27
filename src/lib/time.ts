import type { NowPlaying } from "./activity.svelte";

/**
 * Posición actual de la canción. Rust solo manda cuándo midió la posición; mientras
 * suena, el frontend la avanza con el reloj en vez de recibir un evento por segundo.
 */
export function currentPositionMs(
  track: Pick<NowPlaying, "playing" | "positionMs" | "durationMs" | "sampledAtMs">,
  nowMs: number,
): number {
  const elapsed = track.playing ? Math.max(0, nowMs - track.sampledAtMs) : 0;
  const position = track.positionMs + elapsed;
  return track.durationMs > 0 ? Math.min(position, track.durationMs) : position;
}

/** 83_000 → "1:23"; 3_723_000 → "1:02:03". */
export function formatTime(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`
    : `${minutes}:${seconds}`;
}
