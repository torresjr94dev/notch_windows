import { invoke } from "@tauri-apps/api/core";
import { accentColor, type Rgb } from "./color";

export interface Artwork {
  url: string;
  accent: Rgb | null;
}

/** Lado del canvas donde se reduce la carátula para sacar el acento: suficiente y barato. */
const SAMPLE_SIZE = 24;

// Solo vale la carátula actual (Rust también guarda una sola), pero cachear por id evita
// pedirla otra vez cuando la vista cambia por otra razón (pausa, posición…).
const cache = new Map<string, Promise<Artwork | null>>();

export function loadArtwork(id: string): Promise<Artwork | null> {
  let pending = cache.get(id);
  if (!pending) {
    cache.clear();
    pending = invoke<string | null>("get_artwork", { id }).then(async (url) =>
      url ? { url, accent: await sampleAccent(url) } : null,
    );
    cache.set(id, pending);
  }
  return pending;
}

async function sampleAccent(url: string): Promise<Rgb | null> {
  const image = new Image();
  image.src = url;
  try {
    await image.decode();
  } catch {
    return null;
  }
  const canvas = new OffscreenCanvas(SAMPLE_SIZE, SAMPLE_SIZE);
  const context = canvas.getContext("2d");
  if (!context) return null;
  context.drawImage(image, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
  return accentColor(context.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE).data);
}
