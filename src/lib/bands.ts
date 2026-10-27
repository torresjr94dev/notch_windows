/** Agrupa las bandas del espectro en `count` barras promediando grupos contiguos. */
export function groupBands(levels: readonly number[], count: number): number[] {
  if (count <= 0 || levels.length === 0) return [];
  return Array.from({ length: count }, (_, i) => {
    const start = Math.floor((i * levels.length) / count);
    const end = Math.max(start + 1, Math.floor(((i + 1) * levels.length) / count));
    const group = levels.slice(start, end);
    return group.reduce((sum, v) => sum + v, 0) / group.length;
  });
}
