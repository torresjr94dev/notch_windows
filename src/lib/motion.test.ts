import { describe, expect, it } from "vitest";
import { axisStiffness, visibleRadius } from "./motion";

const compact = { width: 250, height: 36, radius: 18 };
const expanded = { width: 420, height: 176, radius: 42 };

describe("axisStiffness", () => {
  it("al expandir lidera el ancho", () => {
    const { width, height } = axisStiffness(compact, expanded);
    expect(width).toBeGreaterThan(height);
  });

  it("al colapsar lidera el alto", () => {
    const { width, height } = axisStiffness(expanded, compact);
    expect(height).toBeGreaterThan(width);
  });
});

describe("visibleRadius", () => {
  it("no pasa de medio alto", () => {
    expect(visibleRadius(42, 36)).toBe(18);
    expect(visibleRadius(18, 176)).toBe(18);
  });
});
