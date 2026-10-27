export type Rgb = [number, number, number];

/** Fracción de píxeles más vivos que se promedian. */
const TOP_FRACTION = 0.2;
/** Por debajo de esta viveza la carátula es básicamente gris: no hay acento que sacar. */
const MIN_VIBRANCE = 0.12;
/** El glow va sobre negro puro: un acento oscuro no se vería. */
const MIN_LIGHTNESS = 0.6;

/**
 * Color de acento de una carátula (píxeles RGBA de un canvas reducido): el promedio de
 * sus píxeles más vivos, aclarado para que brille sobre negro. `null` si es gris.
 */
export function accentColor(pixels: Uint8ClampedArray): Rgb | null {
  const scored: { score: number; rgb: Rgb }[] = [];
  for (let i = 0; i + 3 < pixels.length; i += 4) {
    if (pixels[i + 3] < 128) continue;
    const rgb: Rgb = [pixels[i], pixels[i + 1], pixels[i + 2]];
    const [, s, l] = toHsl(rgb);
    // Saturación ponderada por qué tan lejos está del negro y del blanco.
    scored.push({ score: s * (1 - Math.abs(2 * l - 1)), rgb });
  }
  if (scored.length === 0) return null;

  scored.sort((a, b) => b.score - a.score);
  const top = scored.slice(0, Math.max(1, Math.ceil(scored.length * TOP_FRACTION)));
  const vibrance = top.reduce((sum, p) => sum + p.score, 0) / top.length;
  if (vibrance < MIN_VIBRANCE) return null;

  const avg = [0, 1, 2].map((c) => top.reduce((sum, p) => sum + p.rgb[c], 0) / top.length) as Rgb;
  const [h, s, l] = toHsl(avg);
  return toRgb([h, s, Math.max(l, MIN_LIGHTNESS)]);
}

function toHsl([r, g, b]: Rgb): [number, number, number] {
  const [rn, gn, bn] = [r / 255, g / 255, b / 255];
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h =
    max === rn
      ? (gn - bn) / d + (gn < bn ? 6 : 0)
      : max === gn
        ? (bn - rn) / d + 2
        : (rn - gn) / d + 4;
  return [h / 6, s, l];
}

function toRgb([h, s, l]: [number, number, number]): Rgb {
  if (s === 0) {
    const v = Math.round(l * 255);
    return [v, v, v];
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const channel = (t: number) => {
    const x = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (x < 1 / 6) return p + (q - p) * 6 * x;
    if (x < 1 / 2) return q;
    if (x < 2 / 3) return p + (q - p) * (2 / 3 - x) * 6;
    return p;
  };
  return [h + 1 / 3, h, h - 1 / 3].map((t) => Math.round(channel(t) * 255)) as Rgb;
}
