import { describe, expect, it } from "vitest";
import { progressLabel } from "./live";

describe("progressLabel", () => {
  it("redondea y acota el progreso", () => {
    expect(progressLabel(0.404)).toBe("40%");
    expect(progressLabel(1.2)).toBe("100%");
    expect(progressLabel(-1)).toBe("0%");
  });

  it("indeterminado no tiene etiqueta", () => {
    expect(progressLabel(null)).toBeNull();
  });
});
