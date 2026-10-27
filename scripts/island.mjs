#!/usr/bin/env node
// Cliente de la API local de Live Activities de Notch Windows.
//
//   node scripts/island.mjs set <id> --title "Build" [--subtitle "main"]
//        [--state running|attention|success|error] [--progress 0.4]
//        [--icon claude|build|deploy|download|test|generic] [--ttl 10]
//   node scripts/island.mjs clear <id>
//
// También se puede importar: `import { setActivity, clearActivity } from "./island.mjs"`.
// Referencia completa: docs/live-activities.md

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

const CONFIG_DIR = join(process.env.APPDATA ?? "", "dev.notchwindows.island");
const DEFAULT_PORT = 47823;
// Corto a propósito: estos scripts corren dentro de hooks y no deben frenar a nadie.
const TIMEOUT_MS = 1500;

function config() {
  const token = readFileSync(join(CONFIG_DIR, "api-token"), "utf8").trim();
  let port = DEFAULT_PORT;
  try {
    port = JSON.parse(readFileSync(join(CONFIG_DIR, "settings.json"), "utf8")).apiPort ?? port;
  } catch {
    // Sin settings.json se usa el puerto por defecto.
  }
  return { token, port };
}

async function send(method, id, body) {
  const { token, port } = config();
  // 127.0.0.1 y no "localhost": la isla solo escucha en IPv4 y "localhost" puede
  // resolver primero a ::1.
  const response = await fetch(`http://127.0.0.1:${port}/v1/activities/${encodeURIComponent(id)}`, {
    method,
    headers: {
      Authorization: `Bearer ${token}`,
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
    signal: AbortSignal.timeout(TIMEOUT_MS),
  });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }
}

/** Crea o reemplaza la actividad `id`. */
export function setActivity(id, activity) {
  return send("PUT", id, activity);
}

/** Quita la actividad `id` de la isla. */
export function clearActivity(id) {
  return send("DELETE", id);
}

async function main() {
  const { positionals, values } = parseArgs({
    allowPositionals: true,
    options: {
      title: { type: "string" },
      subtitle: { type: "string" },
      state: { type: "string" },
      progress: { type: "string" },
      icon: { type: "string" },
      ttl: { type: "string" },
    },
  });
  const [command, id] = positionals;
  if (command === "clear" && id) return clearActivity(id);
  if (command === "set" && id && values.title) {
    return setActivity(id, {
      title: values.title,
      subtitle: values.subtitle,
      state: values.state,
      progress: values.progress === undefined ? undefined : Number(values.progress),
      icon: values.icon,
      ttlSeconds: values.ttl === undefined ? undefined : Number(values.ttl),
    });
  }
  console.error("Uso: island.mjs set <id> --title <texto> [opciones] | island.mjs clear <id>");
  process.exitCode = 2;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  main().catch((err) => {
    console.error(err.message);
    process.exitCode = 1;
  });
}
