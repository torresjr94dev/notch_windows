# ADR-0006: API local de Live Activities

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

Queremos que procesos externos (Claude Code, CI, scripts de deploy) publiquen en la
isla. Hace falta un canal que cualquier lenguaje pueda usar con una línea, sin SDK, y
que no se convierta en una puerta abierta en la máquina del usuario.

## Decisión

### HTTP en `127.0.0.1` con `tiny_http`

- **HTTP + JSON** porque lo habla todo: curl, PowerShell, Node, Python, los hooks.
  Un named pipe sería más cerrado, pero obliga a cada script a usar una API de Windows.
- **`tiny_http`** (síncrono, en su propio hilo) porque son cuatro rutas en localhost.
  Meter `tokio` + `hyper` sumaría tiempo de compilación y binario sin ganar nada.
- **Solo `127.0.0.1`**: ni siquiera escucha en la red local.

### Modelo de amenazas y defensas

| Amenaza                                                         | Defensa                                                                                                                                                                                                  |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Otro equipo de la red escribe en la isla                        | Solo escucha en `127.0.0.1`.                                                                                                                                                                             |
| Una página web hace `fetch` a localhost (CSRF a servicio local) | Se rechaza **cualquier** petición con header `Origin`, que los navegadores mandan siempre en peticiones cross-origin. Además, el header `Authorization` obliga a un preflight CORS que nunca se aprueba. |
| Otro programa del usuario adivina el endpoint                   | Token de 32 bytes del CSPRNG de Windows (`BCryptGenRandom`), comparado en tiempo constante.                                                                                                              |
| Contenido malicioso en la isla (URLs, imágenes, HTML)           | Íconos de un catálogo cerrado, texto plano con longitudes acotadas, `deny_unknown_fields`, cuerpo máximo de 16 KB.                                                                                       |
| Una actividad externa se hace pasar por una del sistema         | Los ids externos llevan el prefijo `live:` y no pueden pisar `media` ni los avisos.                                                                                                                      |

El token vive en `%APPDATA%\dev.notchwindows.island\api-token`, protegido por los
permisos del perfil del usuario. Un programa que ya corre como el usuario puede leerlo:
eso es aceptable, porque ese programa ya podría hacer cosas mucho peores que escribir en
la isla. El objetivo es cerrar el acceso **desde fuera** del usuario (la red y los
navegadores).

### Si el puerto está ocupado

La app arranca sin API y lo registra en el log. La isla no depende de la API. El puerto
se cambia con `apiPort` en `settings.json`.

## Consecuencias

- Integrar algo nuevo es un `PUT` con un JSON pequeño. Hay un cliente de Node
  (`scripts/island.mjs`) y referencia completa en `docs/live-activities.md`.
- El hook de Claude Code (`scripts/claude-hook.mjs`) falla en silencio a propósito: un
  hook roto no debe frenar a Claude. La contra es que sus errores no se ven, como pasó
  con el BOM de PowerShell (corregido).
