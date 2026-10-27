<script lang="ts">
  // La mascota pixel de Claude Code, la del banner de la CLI:
  //
  //    ▐▛███▜▌
  //   ▝▜█████▛▘
  //     ▘▘ ▝▝
  //
  // Cada carácter de bloque son 2×2 pixeles. Las celdas de la terminal miden el doble de
  // alto que de ancho, así que cada pixel se dibuja de 1×2 para respetar la proporción.
  // Los "ojos" son huecos: se ve el negro de la isla a través.

  let { height }: { height: number } = $props();

  /** [x, fila, ancho] de cada tramo horizontal de pixeles. */
  const RUNS: [number, number, number][] = [
    [3, 0, 12],
    [3, 1, 2],
    [6, 1, 6],
    [13, 1, 2],
    [1, 2, 16],
    [3, 3, 12],
    [4, 4, 1],
    [6, 4, 1],
    [11, 4, 1],
    [13, 4, 1],
  ];
  const COLUMNS = 18;
  const ROWS = 5;
</script>

<svg
  viewBox="0 0 {COLUMNS} {ROWS * 2}"
  {height}
  width={(height * COLUMNS) / (ROWS * 2)}
  shape-rendering="crispEdges"
  aria-hidden="true"
>
  {#each RUNS as [x, row, width] (`${x}:${row}`)}
    <rect {x} y={row * 2} {width} height="2" fill="var(--claude-orange)" />
  {/each}
</svg>
