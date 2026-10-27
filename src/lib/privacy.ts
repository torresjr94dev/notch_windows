import type { Privacy } from "./activity.svelte";

/** Texto para lectores de pantalla: el color del indicador no puede ser la única pista. */
export function privacyLabel({ microphone, camera }: Privacy): string {
  const parts: string[] = [];
  if (microphone.length) parts.push(`Micrófono en uso por ${list(microphone)}`);
  if (camera.length) parts.push(`Cámara en uso por ${list(camera)}`);
  return parts.join(". ");
}

function list(apps: string[]): string {
  return apps.length > 1 ? `${apps.slice(0, -1).join(", ")} y ${apps.at(-1)}` : apps[0];
}
