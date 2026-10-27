import { describe, expect, it } from "vitest";
import { accentColor } from "./color";

function image(...pixels: [number, number, number, number][]): Uint8ClampedArray {
  return new Uint8ClampedArray(pixels.flat());
}

describe("accentColor", () => {
  it("una carátula gris no tiene acento", () => {
    expect(accentColor(image([40, 40, 40, 255], [200, 200, 200, 255]))).toBeNull();
  });

  it("toma el color vivo aunque domine el gris", () => {
    const gray: [number, number, number, number] = [90, 90, 90, 255];
    const accent = accentColor(image(gray, gray, gray, gray, [220, 30, 30, 255]));
    expect(accent).not.toBeNull();
    const [r, g, b] = accent!;
    expect(r).toBeGreaterThan(g + 80);
    expect(r).toBeGreaterThan(b + 80);
  });

  it("aclara los acentos oscuros para que brillen sobre negro", () => {
    const [r, g, b] = accentColor(image([10, 20, 120, 255]))!;
    const lightness = (Math.max(r, g, b) + Math.min(r, g, b)) / 2 / 255;
    expect(lightness).toBeGreaterThanOrEqual(0.59);
    expect(b).toBeGreaterThan(r);
  });

  it("ignora píxeles transparentes", () => {
    expect(accentColor(image([255, 0, 0, 0]))).toBeNull();
  });
});
