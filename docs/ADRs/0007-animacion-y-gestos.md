# ADR-0007: Animación (springs por eje, split gooey) y gestos

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

La sensación de "Dynamic Island" depende casi por completo del movimiento: la pastilla
cambia de forma con física de resorte, y una segunda actividad "gotea" fuera de ella.
También queremos interacciones rápidas sin abrir nada (volumen, cambiar de canción).

## Decisiones

### Animamos `width`/`height`, no `transform` (excepción consciente)

La regla general de rendimiento es animar solo `transform` y `opacity`. Aquí no aplica:

- La pastilla cambia de **forma**, no solo de tamaño: el radio de las esquinas y la
  proporción cambian a la vez. `scale()` deformaría las esquinas y el contenido.
- El árbol que se re-maqueta es diminuto (una pastilla con un puñado de nodos) y está
  aislado en posición absoluta, así que el costo de layout por cuadro es despreciable.
- Dentro de la pastilla **sí** se respeta la regla: el visualizador, las barras de
  progreso, el anillo y el empujón de los gestos usan `transform`.

### Un spring por eje

Ancho, alto y radio tienen springs propios. Al expandir, el ancho lidera (más rígido) y
el alto lo sigue; al colapsar, al revés (`lib/motion.ts`). Con un solo spring, la
pastilla "infla" como globo. El radio visible nunca pasa de medio alto. Con "Efectos de
animación" de Windows apagados (`prefers-reduced-motion`), todo es instantáneo.

### Split gooey en dos capas

La burbuja de la actividad secundaria sale de detrás de la pastilla y se separa. El
efecto líquido es un filtro SVG (blur + umbral de alfa + `atop`) aplicado a una **capa de
formas** (solo el negro). El contenido va en otra capa encima, sin filtro, porque el
blur emborronaría el texto. El filtro se activa solo mientras hay burbuja, así que en
reposo no cuesta nada. El glow pasó de `box-shadow` a `drop-shadow` sobre esa misma
capa, para que siga la silueta fundida de pastilla y burbuja.

### Gestos, siempre con alternativa visible

| Gesto                            | Acción                        | Alternativa visible               |
| -------------------------------- | ----------------------------- | --------------------------------- |
| Rueda vertical sobre la isla     | Volumen ±2% por muesca        | Teclas de volumen, OSD de la isla |
| Swipe horizontal o Shift + rueda | Siguiente / anterior canción  | Botones de Now Playing expandido  |
| Clic en la pastilla              | Expandir sin esperar el hover | Hover                             |

- Los deltas se convierten en pasos con un acumulador (`lib/gestures.ts`): los
  touchpads mandan deltas chicos y continuos, los mouses saltos de ~100 px.
- El swipe de canciones tiene un enfriamiento de 700 ms: la inercia del touchpad no debe
  saltar tres canciones de golpe. Un empujón lateral confirma que el gesto se registró.
- El volumen se ajusta en Rust, en el hilo del provider (dueño del endpoint de Core
  Audio), acotado a ±10% por comando.
- La rueda llega a la isla aunque no tenga foco gracias a "Desplazar ventanas inactivas"
  de Windows (activo por defecto). Se verificó en ejecución.

## Consecuencias

- Las animaciones de forma cuestan algo de layout por cuadro. Medido en la práctica: no
  se nota, y el proceso principal se mantiene en ~30 MB.
- Si algún día la pastilla tiene contenido pesado, conviene revisar esta decisión (por
  ejemplo, animar un `clip-path` sobre un contenedor de tamaño fijo).
