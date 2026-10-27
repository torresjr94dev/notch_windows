import { describe, expect, it } from "vitest";
import { privacyLabel } from "./privacy";

describe("privacyLabel", () => {
  it("vacío sin apps", () => {
    expect(privacyLabel({ microphone: [], camera: [] })).toBe("");
  });

  it("nombra las apps de cada dispositivo", () => {
    expect(privacyLabel({ microphone: ["Teams"], camera: [] })).toBe("Micrófono en uso por Teams");
    expect(privacyLabel({ microphone: ["Discord", "OBS", "Teams"], camera: ["Teams"] })).toBe(
      "Micrófono en uso por Discord, OBS y Teams. Cámara en uso por Teams",
    );
  });
});
