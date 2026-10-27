# ADR-0004: Motor de actividades: providers, árbitro y vista

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

La isla muestra cosas de fuentes muy distintas: música (GSMTC), avisos de un par de
segundos (volumen, brillo, Bloq Mayús), procesos externos (Claude Code, CI), etc. Varias
pueden estar activas al mismo tiempo y compiten por un solo espacio. Sin una regla
central, cada fuente se pelearía por la pastilla.

## Decisión

### Providers → Hub → Árbitro → Vista

```
providers (un hilo cada uno) ──upsert/remove──▶ ActivityHub (mpsc)
                                                     │
                                         hilo árbitro (Board, lógica pura)
                                                     │ island:view (solo si cambió)
                                                     ▼
                                      frontend de cada isla (solo dibuja)
```

- Una **actividad** tiene un `id` (publicar el mismo id la reemplaza), una `priority`,
  una marca `transient` y un `ttl` opcional.
- El **árbitro** (`activity::board`) elige:
  - **Primaria**: la de mayor prioridad; en empate gana la más reciente.
  - **Secundaria** (burbuja del split): la siguiente que no sea transitoria ni del mismo
    tipo que la primaria. Mientras dura un aviso de volumen, la música pasa a la burbuja.
- El hilo **duerme** hasta que llega una actualización o vence el siguiente TTL
  (`recv_timeout`): en reposo no consume nada.
- Republicar una actividad idéntica **no reinicia su TTL**. Así una canción en pausa
  caduca a los 5 minutos aunque Windows siga reportando la pausa cada segundo.

### Lo que no viaja en la vista

- **Carátula:** viaja por id. Los bytes (data URL) se piden una vez con `get_artwork`.
  Reenviarlos en cada cambio de la vista mandaría ~100 KB a cada isla por cada pausa.
- **Posición de la canción:** Rust manda `positionMs` y cuándo la midió
  (`sampledAtMs`), y el frontend la interpola con su reloj. Emitir un evento por segundo
  por cada isla no aporta nada.
- **Espectro:** va por su propio evento (`island:spectrum`, ~30 fps, 16 bytes), separado
  de la vista para no recalcular el arbitraje 30 veces por segundo.

### Visualizador y privacidad

El visualizador captura la **salida** de audio (WASAPI loopback), no el micrófono, así
que no activa el indicador de privacidad de Windows. Solo corre mientras hay música
reproduciéndose, y el usuario lo puede apagar desde la bandeja.

## Alternativas descartadas

- **Que cada provider emita directo al frontend:** el arbitraje terminaría duplicado en
  JS, por ventana, y los TTL dependerían de timers del WebView.
- **Prioridad fija por tipo:** no alcanza. "Claude necesita permiso" debe ganarle a la
  música, pero "Claude trabajando" no.

## Consecuencias

- Agregar una fuente nueva es: un provider que publique en el hub, una variante en
  `ActivityData` (Rust) y en `src/lib/activity.svelte.ts` (TS), y sus componentes.
- Los tipos de Rust y TS se mantienen a mano y en sync. Si se vuelven muchos, convendrá
  generar los de TS (p. ej. con `ts-rs`).
