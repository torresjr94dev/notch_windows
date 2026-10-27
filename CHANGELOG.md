# Changelog

Todos los cambios relevantes de este proyecto se documentan aquí. El formato sigue
[Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el proyecto usa
[Versionado Semántico](https://semver.org/lang/es/).

## [Sin publicar]

### Añadido

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
