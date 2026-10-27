import { describe, expect, it } from "vitest";
import { createAccumulator, wheelPixels } from "./gestures";

function recorder(options: { threshold: number; cooldownMs?: number }) {
  const steps: number[] = [];
  const push = createAccumulator({ ...options, onStep: (d) => steps.push(d) });
  return { steps, push };
}

describe("createAccumulator", () => {
  it("una muesca de mouse es un paso", () => {
    const { steps, push } = recorder({ threshold: 100 });
    push(-100, 0);
    push(100, 50);
    expect(steps).toEqual([-1, 1]);
  });

  it("los deltas chicos de touchpad se suman", () => {
    const { steps, push } = recorder({ threshold: 100 });
    for (let t = 0; t < 10; t++) push(15, t * 16);
    expect(steps).toEqual([1]);
  });

  it("un giro grande da varios pasos", () => {
    const { steps, push } = recorder({ threshold: 100 });
    push(250, 0);
    expect(steps).toEqual([1, 1]);
  });

  it("descarta lo acumulado tras una pausa", () => {
    const { steps, push } = recorder({ threshold: 100 });
    push(60, 0);
    push(60, 1000);
    expect(steps).toEqual([]);
  });

  it("con enfriamiento, la inercia no encadena pasos", () => {
    const { steps, push } = recorder({ threshold: 100, cooldownMs: 500 });
    for (let t = 0; t < 30; t++) push(40, t * 16);
    expect(steps).toEqual([1]);
    for (let t = 0; t < 5; t++) push(40, 900 + t * 16);
    expect(steps).toEqual([1, 1]);
  });
});

describe("wheelPixels", () => {
  it("normaliza líneas y páginas a píxeles", () => {
    expect(wheelPixels(3, 1)).toBe(120);
    expect(wheelPixels(100, 0)).toBe(100);
    expect(wheelPixels(1, 2)).toBe(800);
  });
});
