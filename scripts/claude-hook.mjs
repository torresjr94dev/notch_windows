#!/usr/bin/env node
// Hook de Claude Code → isla. Claude Code le pasa el evento como JSON por stdin; este
// script publica el estado de la sesión como Live Activity:
//
//   UserPromptSubmit         → "Claude trabajando"
//   PostToolUse              → sigue trabajando; el subtítulo dice qué hizo ("Edit · App.svelte")
//   Notification (permiso)   → "Claude te necesita" (atención: le gana a la música)
//   Stop                     → "Claude terminó" (se va sola en 8 s)
//   StopFailure              → "Claude se detuvo" (error de la API; se va en 10 s)
//   SessionEnd               → se quita
//
// Cada publicación manda el PID del proceso padre del hook: al hacer clic en la isla,
// Rust sube por el árbol de procesos hasta la terminal donde corre esta sesión.
//
// Nunca debe frenar ni romper a Claude: si la isla no está corriendo o algo falla,
// termina en silencio con código 0. Configuración: docs/live-activities.md

import { basename } from "node:path";
import { clearActivity, setActivity } from "./island.mjs";

const MAX_SUBTITLE = 120;
/** Si interrumpes a Claude (Esc) no corre ningún hook: "trabajando" caduca solo si no
 *  hay actividad. Cada herramienta cambia el subtítulo y reinicia la cuenta. */
const RUNNING_TTL_SECONDS = 10 * 60;

async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  // Algunos shells (PowerShell con salida UTF-8) anteponen un BOM que JSON.parse rechaza.
  return Buffer.concat(chunks).toString("utf8").replace(/^﻿/, "");
}

function clip(text) {
  if (!text) return undefined;
  const line = String(text).replace(/\s+/g, " ").trim();
  return line.length > MAX_SUBTITLE ? `${line.slice(0, MAX_SUBTITLE - 1)}…` : line;
}

/** "Edit · App.svelte", "Bash · npm run lint", "Grep · TODO"… */
function describeTool(name, input = {}) {
  const file = input.file_path ?? input.notebook_path ?? input.path;
  if (file) return `${name} · ${basename(String(file))}`;
  if (input.command) return `${name} · ${String(input.command).split("\n")[0]}`;
  if (input.pattern) return `${name} · ${input.pattern}`;
  return name;
}

/** Avisos que no piden nada: "Claude está esperando tu input" llega 60 s después de
 *  terminar y dejaría la isla en ámbar hasta el siguiente prompt. */
function isIdleReminder(input) {
  return (
    input.notification_type === "idle_prompt" || /waiting for your input/i.test(input.message ?? "")
  );
}

try {
  const input = JSON.parse(await readStdin());
  // Una actividad por sesión: varias sesiones a la vez se ven por separado.
  const id = `claude-${String(input.session_id ?? "local").slice(0, 8)}`;
  const project = input.cwd ? basename(input.cwd) : undefined;
  const base = { icon: "claude", focusPid: process.ppid };

  switch (input.hook_event_name) {
    case "UserPromptSubmit":
      await setActivity(id, {
        ...base,
        title: "Claude trabajando",
        subtitle: clip(project),
        state: "running",
        ttlSeconds: RUNNING_TTL_SECONDS,
      });
      break;
    case "PostToolUse":
      await setActivity(id, {
        ...base,
        title: "Claude trabajando",
        subtitle: clip(
          [project, describeTool(input.tool_name, input.tool_input)].filter(Boolean).join(" · "),
        ),
        state: "running",
        ttlSeconds: RUNNING_TTL_SECONDS,
      });
      break;
    case "Notification":
      if (isIdleReminder(input)) break;
      await setActivity(id, {
        ...base,
        title: "Claude te necesita",
        subtitle: clip(input.message ?? project),
        state: "attention",
      });
      break;
    case "Stop":
      await setActivity(id, {
        ...base,
        title: "Claude terminó",
        subtitle: clip(project),
        state: "success",
        ttlSeconds: 8,
      });
      break;
    case "StopFailure":
      await setActivity(id, {
        ...base,
        title: "Claude se detuvo",
        subtitle: clip(input.error ?? input.message ?? project),
        state: "error",
        ttlSeconds: 10,
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
