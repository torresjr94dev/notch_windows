# Changelog

Todos los cambios relevantes de este proyecto se documentan aquí. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto usa
[Versionado Semántico](https://semver.org/lang/es/).

## [Sin publicar]

## [0.1.0] - 2026-10-27

Primera versión publicada, con instalador para Windows.

### Añadido

- Instalador NSIS en español: se instala para tu usuario, sin permisos de
  administrador.
- Ícono propio de la app.
- Una sola instancia: abrir la app otra vez muestra la isla si estaba oculta, en vez de
  crear una segunda.
- Proyecto base con Tauri v2, Svelte 5 y Vite.
- Ventana de la isla transparente, siempre encima y anclada arriba al centro del monitor
  principal. Se reubica sola si cambian la resolución, el DPI o la barra de tareas.
- La isla nunca roba el foco y no aparece en la barra de tareas ni en Alt+Tab.
- Click-through: los clics fuera de la pastilla pasan a las ventanas de abajo.
- La isla se oculta con juegos, videos o presentaciones en pantalla completa.
- Icono en la bandeja para ocultar o mostrar la isla y para salir.
- Pastilla animada con springs entre modo compacto y expandido, con un reloj como
  actividad por defecto. Respeta la opción "Efectos de animación" de Windows.
- `npm run lint` (Prettier, svelte-check, rustfmt, clippy) y `npm test`.
- ADR-0001 (stack) y ADR-0002 (ventana y click-through).
- Selección de pantalla desde la bandeja: el monitor principal, un monitor concreto
  (identificado por su nombre de modelo) o todas las pantallas. Las islas se crean, mueven
  y destruyen solas al conectar o desconectar monitores.
- La pantalla completa se detecta por monitor: un juego en un monitor ya no esconde la
  isla de los demás.
- Preferencias persistidas en `settings.json`.
- ADR-0003 (una ventana por monitor).
- Motor de actividades: los providers publican y un árbitro decide qué va en la pastilla
  y qué en la burbuja del split, con prioridades y TTL.
- Now Playing desde los controles de medios de Windows (GSMTC): carátula, título,
  artista, progreso con seek y controles de reproducción.
- Glow, barras y progreso teñidos con el color de acento de la carátula.
- Visualizador de audio (WASAPI loopback + FFT), activable desde la bandeja.
- Tests de frontend con vitest; `npm test` corre vitest y cargo test.
- Fixture `fake_media` para probar Now Playing sin música.
- ADR-0004 (motor de actividades).
- Avisos transitorios de volumen (callback de Core Audio), brillo (WMI, laptops),
  cargador y batería baja, y Bloq Mayús.
- Burbuja de privacidad cuando una app usa el micrófono o la cámara.
- ADR-0005 (avisos del sistema y por qué no se oculta el OSD nativo).
- API local de Live Activities en `127.0.0.1` con token, rechazo de navegadores y
  validación estricta; puerto configurable.
- Integración con Claude Code vía hooks y seguidor de CI de GitHub Actions
  (`scripts/`).
- Opción de bandeja para abrir la carpeta de configuración.
- `docs/live-activities.md` y ADR-0006 (API local).
- Split líquido: la actividad secundaria sale de la pastilla como burbuja (filtro
  gooey).
- Springs por eje: al expandir lidera el ancho y al colapsar el alto.
- Gestos: rueda sobre la isla para el volumen, swipe horizontal o Shift + rueda para
  cambiar de canción, feedback de presión.
- ADR-0007 (animación y gestos).
- Opción _Iniciar con Windows_ en la bandeja, consistente con el Administrador de
  tareas.
- Clic en una Live Activity lleva al frente su ventana de origen (`focusPid`): "Claude te
  necesita" te deja en la terminal de esa sesión.
- Logo pixel de Claude Code en sus actividades.
- El hook de Claude Code sigue el ciclo completo: vuelve a "trabajando" tras aprobar un
  permiso (con la herramienta en uso), marca errores de la API y no se queda pegado al
  interrumpir.
- Workflow de release: un tag `vX.Y.Z` compila el instalador en GitHub Actions y
  publica el release con las notas de este archivo y el SHA-256.
- ADR-0008 (distribución: instalador, firma y actualizaciones).

### Corregido

- _Iniciar con Windows_ se marcaba como activo aunque el arranque apuntara a otra copia
  de la app (por ejemplo, la de `tauri dev`).

[Sin publicar]: https://github.com/torresjr94dev/notch_windows/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/torresjr94dev/notch_windows/releases/tag/v0.1.0
