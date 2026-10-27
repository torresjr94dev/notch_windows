# Notch Windows

La "Dynamic Island" de Apple, pero para Windows: una pastilla negra arriba al centro de
la pantalla que crece con animaciones de resorte para mostrar actividades en vivo.

> **Estado:** base del proyecto. La ventana, el click-through, el ocultado en pantalla
> completa, la bandeja y la animación compacto ↔ expandido ya funcionan. La primera
> actividad real ("Now Playing") es lo siguiente. Ver [Roadmap](#roadmap).

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
npm test               # tests unitarios de Rust
npm run format         # aplica Prettier y rustfmt
```

Una vez abierta, la app no tiene ventana principal ni pestaña en la barra de tareas. Se
controla desde el **icono en la bandeja**:

- _Ocultar/Mostrar isla_.
- _Pantalla_: el monitor principal de Windows, un monitor concreto (aparece con su
  nombre de modelo) o _Todas las pantallas_.
- _Salir_.

Las preferencias se guardan en `%APPDATA%\dev.notchwindows.island\settings.json`.

## Arquitectura

```
┌─ Rust (src-tauri) ─────────────────────────────────────┐
│ island::watcher   hilo vigilante (30 Hz)                │
│   ├─ monitores (500 ms) → crear / reasignar / destruir  │
│   ├─ hit-test del cursor → click-through + island:hover │
│   ├─ pantalla completa por monitor → ocultar / mostrar  │
│   └─ anclaje arriba al centro de cada monitor           │
│ island::displays  reglas de monitores, puras y con tests│
│ island::win32     todo el FFI con Win32 (único unsafe)  │
│ island::geometry  geometría pura, con tests             │
│ settings          preferencias en settings.json         │
│ tray              icono y menú de bandeja               │
└──────────────┬──────────────────────▲──────────────────┘
       evento  │ island:hover         │ comando set_hit_rect
               ▼                      │
┌─ Frontend (src, WebView2 + Svelte 5) ──────────────────┐
│ App.svelte          modos compacto/expandido + Spring   │
│ lib/hitArea.ts      reporta el área de la pastilla      │
│ lib/hoverIntent.ts  retardos de entrada y salida        │
└────────────────────────────────────────────────────────┘
```

Regla de oro: **Rust es la fuente de verdad y el frontend solo dibuja.** Las decisiones
de fondo están documentadas en:

- [ADR-0001: Tauri v2 + Svelte 5 como stack](docs/ADRs/0001-stack-tauri-v2.md)
- [ADR-0002: Ventana fija, click-through y convivencia con tao](docs/ADRs/0002-ventana-fija-y-click-through.md).
  **Léelo antes de tocar cualquier cosa de la ventana.**
- [ADR-0003: Una ventana por monitor, creada en runtime](docs/ADRs/0003-una-ventana-por-monitor.md)

## Estructura

```
├── src/                     frontend (Svelte 5 + TS)
│   ├── App.svelte
│   └── lib/
├── src-tauri/               backend (Rust)
│   ├── src/island/          ventanas de la isla (una por monitor)
│   ├── src/settings.rs      preferencias persistidas
│   ├── src/tray.rs
│   ├── capabilities/        permisos de Tauri
│   └── tauri.conf.json
├── docs/ADRs/               decisiones de arquitectura
└── CHANGELOG.md
```

## Roadmap

- **MVP:** _Now Playing_ (GSMTC: carátula, controles y progreso), indicador de volumen y
  brillo, y animación de carga de batería.
- **Fase 2:** indicador de mic/cámara en uso, timer/Pomodoro, próximo evento del
  calendario, audífonos Bluetooth con batería y un "shelf" para arrastrar archivos.
- **Fase 3:** notificaciones (requiere empaquetar como MSIX) y una API local de _Live
  Activities_ para que scripts y apps externas publiquen en la isla.

## Convenciones

- Git Flow: una rama `tipo/descripcion-corta` por cambio y un PR hacia `main`. Nunca se
  hace commit directo a `main`.
- [Conventional Commits](https://www.conventionalcommits.org/es/v1.0.0/) con commits atómicos.
- Código en inglés; documentación, commits y PRs en español.
- Actualizar el [CHANGELOG](CHANGELOG.md) en cada cambio visible.
