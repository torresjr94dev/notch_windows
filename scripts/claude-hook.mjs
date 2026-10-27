#!/usr/bin/env node
// Hook de Claude Code → isla. Claude Code le pasa el evento como JSON por stdin; este
// script publica el estado de la sesión como Live Activity:
//
//   UserPromptSubmit → "Claude trabajando"   (corriendo)
//   Notification     → "Claude te necesita"  (atención: le gana a la música)
//   Stop             → "Claude terminó"      (éxito, se va sola en 8 s)
//   SessionEnd       → se quita
//
// Nunca debe frenar ni romper a Claude: si la isla no está corriendo o algo falla,
// termina en silencio con código 0. Configuración: docs/live-activities.md

import { basename } from "node:path";
import { clearActivity, setActivity } from "./island.mjs";

const MAX_SUBTITLE = 120;

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString("utf8");
}

function clip(text) {
  if (!text) return undefined;
  return text.length > MAX_SUBTITLE ? `${text.slice(0, MAX_SUBTITLE - 1)}…` : text;
}

try {
  const input = JSON.parse(await readStdin());
  // Una actividad por sesión: varias sesiones a la vez se ven por separado.
  const id = `claude-${String(input.session_id ?? "local").slice(0, 8)}`;
  const project = input.cwd ? basename(input.cwd) : undefined;

  switch (input.hook_event_name) {
    case "UserPromptSubmit":
      await setActivity(id, {
        title: "Claude trabajando",
        subtitle: clip(project),
        state: "running",
        icon: "claude",
      });
      break;
    case "Notification":
      await setActivity(id, {
        title: "Claude te necesita",
        subtitle: clip(input.message ?? project),
        state: "attention",
        icon: "claude",
      });
      break;
    case "Stop":
      await setActivity(id, {
        title: "Claude terminó",
        subtitle: clip(project),
        state: "success",
        icon: "claude",
        ttlSeconds: 8,
      });
      break;
    case "SessionEnd":
      await clearActivity(id);
      break;
  }
} catch {
  // Isla apagada, sin token o entrada inesperada: no es asunto de Claude.
}
process.exit(0);
