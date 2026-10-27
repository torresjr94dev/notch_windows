import { describe, expect, it } from "vitest";
import type { Activity } from "./activity.svelte";
import { focusTarget, progressLabel } from "./live";

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

describe("focusTarget", () => {
  const live = (focusable: boolean): Activity => ({
    id: "live:claude-1",
    priority: 90,
    transient: false,
    data: {
      kind: "live",
      title: "Claude te necesita",
      subtitle: null,
      state: "attention",
      progress: null,
      icon: "claude",
      focusable,
    },
  });

  it("solo las Live Activities con ventana de origen llevan a algún lado", () => {
    expect(focusTarget(live(true))).toBe("live:claude-1");
    expect(focusTarget(live(false))).toBeNull();
    expect(focusTarget(null)).toBeNull();
  });
});
