import { describe, expect, it } from "vitest";
import type { Activity } from "./activity.svelte";
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

describe("layout", () => {
  it("sin actividades muestra la isla vacía o el reloj", () => {
    const empty = { primary: null, secondary: null };
    expect(layout(empty, false)).toEqual({ face: "idle", mode: "compact", activity: null });
    expect(layout(empty, true).mode).toBe("expanded");
  });

  it("el hover expande la actividad principal", () => {
    const view = { primary: music, secondary: null };
    expect(layout(view, true)).toEqual({ face: "nowPlaying", mode: "expanded", activity: music });
  });

  it("un transitorio gana aunque haya hover y nunca se expande", () => {
    const volume = { ...music, id: "volume", transient: true };
    expect(layout({ primary: volume, secondary: music }, true)).toEqual({
      face: "nowPlaying",
      mode: "compact",
      activity: volume,
    });
  });
});
