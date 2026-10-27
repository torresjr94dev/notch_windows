# Notch Windows

La "Dynamic Island" de Apple, pero para Windows: una pastilla negra arriba al centro de
la pantalla que crece con animaciones de resorte para mostrar actividades en vivo.

## Qué hace

- **Now Playing:** lo que suena en cualquier app que se reporte a Windows (Spotify,
  navegadores, Reproductor multimedia…). En compacto muestra la carátula y un
  visualizador de audio; al pasar el cursor (o con un clic) se expande con título,
  artista, progreso con seek y controles. El glow toma el color de la carátula.
- **Una isla por monitor:** en el principal, en uno concreto o en todos.
- **Nunca estorba:** fuera de la pastilla los clics pasan a lo que hay debajo, nunca le
  roba el foco a nadie, no aparece en la barra de tareas ni en Alt+Tab, y se esconde
  cuando su monitor tiene algo en pantalla completa.

## Requisitos

| Herramienta                | Versión                                     | Notas                                                                       |
| -------------------------- | ------------------------------------------- | --------------------------------------------------------------------------- |
| Windows                    | 10 u 11                                     | La app es solo para Windows.                                                |
| [Rust](https://rustup.rs/) | estable, toolchain `x86_64-pc-windows-msvc` |                                                                             |
| Visual Studio Build Tools  | 2022 o más reciente                         | Con el workload **"Desarrollo de escritorio con C++"** (provee `link.exe`). |
| Node.js                    | 20 o más reciente                           | Probado con Node 24.                                                        |
| WebView2 Runtime           | evergreen                                   | Ya viene en Windows 11.                                                     |

No se necesitan **variables de entorno**.

## Instalación y uso

```bash
npm install            # dependencias del frontend y la CLI de Tauri
npm run tauri dev      # app en desarrollo con HMR (la primera compilación de Rust tarda)
npm run tauri build    # instalador de producción en src-tauri/target/release/bundle
```

Antes de abrir un PR:

```bash
npm run lint           # Prettier + svelte-check + rustfmt + clippy (-D warnings)
npm test               # vitest (frontend) + cargo test (Rust)
npm run format         # aplica Prettier y rustfmt
```

Para probar Now Playing sin poner música, hay un fixture que reproduce una "canción" en
silencio registrada en los controles de medios de Windows:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --example fake_media
```

Una vez abierta, la app no tiene ventana principal ni pestaña en la barra de tareas. Se
controla desde el **icono en la bandeja**:

- _Ocultar/Mostrar isla_.
- _Pantalla_: el monitor principal de Windows, un monitor concreto (aparece con su
  nombre de modelo) o _Todas las pantallas_.
- _Visualizador de audio_: captura la salida de audio (nunca el micrófono) solo mientras
  hay música sonando.
- _Salir_.

Las preferencias se guardan en `%APPDATA%\dev.notchwindows.island\settings.json`.

## Arquitectura

```
┌─ Rust (src-tauri) ─────────────────────────────────────────┐
│ providers::media     GSMTC → Now Playing + controles        │
│ providers::spectrum  WASAPI loopback + FFT → island:spectrum│
│        │ upsert/remove                                      │
│        ▼                                                    │
│ activity             árbitro: prioridad, split, TTL         │
│        │ island:view (solo si cambió)                       │
│ island::watcher      hilo vigilante (30 Hz)                 │
│   ├─ monitores → crear / reasignar / destruir islas         │
│   ├─ hit-test del cursor → click-through + island:hover     │
│   ├─ pantalla completa por monitor → ocultar / mostrar      │
│   └─ anclaje arriba al centro de cada monitor               │
│ island::win32        todo el FFI de ventanas (único unsafe) │
│ settings · tray      preferencias y menú de bandeja         │
└──────────────┬─────────────────────────▲───────────────────┘
       eventos │                         │ comandos
               ▼                         │ set_hit_rect, get_view,
┌─ Frontend (src, WebView2 + Svelte 5) ──┴ media_control… ────┐
│ App.svelte              forma (Spring) + contenido por modo │
│ lib/layout.ts           qué cara y modo mostrar (puro)      │
│ lib/activity.svelte.ts  estado compartido de la isla        │
│ components/             Now Playing, visualizador, reloj    │
└─────────────────────────────────────────────────────────────┘
```

Regla de oro: **Rust es la fuente de verdad y el frontend solo dibuja.** Las decisiones
de fondo están documentadas en:

- [ADR-0001: Tauri v2 + Svelte 5 como stack](docs/ADRs/0001-stack-tauri-v2.md)
- [ADR-0002: Ventana fija, click-through y convivencia con tao](docs/ADRs/0002-ventana-fija-y-click-through.md).
  **Léelo antes de tocar cualquier cosa de la ventana.**
- [ADR-0003: Una ventana por monitor, creada en runtime](docs/ADRs/0003-una-ventana-por-monitor.md)
- [ADR-0004: Motor de actividades](docs/ADRs/0004-motor-de-actividades.md). Explica
  cómo agregar una fuente nueva.

## Estructura

```
├── src/                     frontend (Svelte 5 + TS)
│   ├── App.svelte
│   ├── components/          piezas visuales
│   └── lib/                 lógica (con *.test.ts de vitest)
├── src-tauri/               backend (Rust)
│   ├── src/activity/        modelo de actividades y árbitro
│   ├── src/providers/       fuentes: medios, espectro
│   ├── src/island/          ventanas de la isla (una por monitor)
│   ├── src/settings.rs      preferencias persistidas
│   ├── src/tray.rs
│   ├── examples/            fixtures de desarrollo
│   ├── capabilities/        permisos de Tauri
│   └── tauri.conf.json
├── docs/ADRs/               decisiones de arquitectura
└── CHANGELOG.md
```

## Roadmap

- **Sistema:** volumen y brillo, carga de batería, indicador de mic/cámara en uso,
  Bloq Mayús.
- **Live Activities:** API local para que scripts y apps publiquen en la isla
  (Claude Code, CI, deploys).
- **Interacción:** split líquido con dos actividades, gestos (rueda = volumen, swipe =
  cambiar canción), física por eje.
- **Después:** timer/Pomodoro, calendario, audífonos Bluetooth, shelf de archivos,
  notificaciones (requiere MSIX).

## Convenciones

- Git Flow: una rama `tipo/descripcion-corta` por cambio y un PR hacia `main`. Nunca se
  hace commit directo a `main`.
- [Conventional Commits](https://www.conventionalcommits.org/es/v1.0.0/) con commits atómicos.
- Código en inglés; documentación, commits y PRs en español.
- Actualizar el [CHANGELOG](CHANGELOG.md) en cada cambio visible.
