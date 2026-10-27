import { describe, expect, it } from "vitest";
import { currentPositionMs, formatTime } from "./time";

describe("currentPositionMs", () => {
  const track = { playing: true, positionMs: 10_000, durationMs: 60_000, sampledAtMs: 1_000 };

  it("avanza con el reloj mientras suena", () => {
    expect(currentPositionMs(track, 4_000)).toBe(13_000);
  });

  it("se queda quieta en pausa", () => {
    expect(currentPositionMs({ ...track, playing: false }, 4_000)).toBe(10_000);
  });

  it("no se pasa de la duración", () => {
    expect(currentPositionMs(track, 999_000)).toBe(60_000);
  });

  it("ignora relojes que van hacia atrás", () => {
    expect(currentPositionMs(track, 0)).toBe(10_000);
  });
});

describe("formatTime", () => {
  it.each([
    [0, "0:00"],
    [83_000, "1:23"],
    [3_723_000, "1:02:03"],
    [-5, "0:00"],
  ])("%i ms → %s", (ms, text) => {
    expect(formatTime(ms)).toBe(text);
  });
});
