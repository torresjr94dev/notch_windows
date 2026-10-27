import { describe, expect, it } from "vitest";
import type { Activity, IslandView } from "./activity.svelte";
import { layout } from "./layout";

const music: Activity = {
  id: "media",
  priority: 50,
  transient: false,
  data: {
    kind: "nowPlaying",
    title: "Song",
    artist: "Artist",
    album: "",
    app: "Spotify",
    artworkId: null,
    playing: true,
    positionMs: 0,
    durationMs: 0,
    sampledAtMs: 0,
  },
};

const volume: Activity = {
  id: "system:volume",
  priority: 100,
  transient: true,
  data: { kind: "volume", level: 0.4, muted: false },
};

function view(primary: Activity | null, secondary: Activity | null = null): IslandView {
  return { primary, secondary, privacy: { microphone: [], camera: [] } };
}

describe("layout", () => {
  it("sin actividades muestra la isla vacía o el reloj", () => {
    expect(layout(view(null), false)).toEqual({ face: "idle", mode: "compact", activity: null });
    expect(layout(view(null), true).mode).toBe("expanded");
  });

  it("el hover expande la actividad principal", () => {
    expect(layout(view(music), true)).toEqual({
      face: "nowPlaying",
      mode: "expanded",
      activity: music,
    });
  });

  it("un transitorio gana aunque haya hover y nunca se expande", () => {
    expect(layout(view(volume, music), true)).toEqual({
      face: "volume",
      mode: "compact",
      activity: volume,
    });
  });
});
