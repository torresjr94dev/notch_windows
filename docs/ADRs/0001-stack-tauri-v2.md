# ADR-0001: Tauri v2 (Rust) + Svelte 5 como stack

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

Notch Windows es una "Dynamic Island" para Windows: una pastilla siempre visible arriba
al centro de la pantalla que muestra actividades en vivo (música, volumen, batería,
timers, etc.). El stack tiene que cumplir tres requisitos:

1. **Animaciones orgánicas** (física de resorte) a la tasa de refresco del monitor.
   Sin ellas, la app se siente como un widget genérico.
2. **Acceso profundo a APIs de Windows**: WinRT (`GlobalSystemMediaTransportControls`,
   `UserNotificationListener`), Core Audio y Win32 (estilos de ventana, cursor, monitores).
3. **Bajo consumo en reposo**, porque la app vive encendida todo el día.

## Decisión

- **Backend:** Rust con Tauri v2. El acceso a Windows se hace con el crate `windows`,
  fijado a la misma versión que usa Tauri internamente (hoy 0.62) para compartir el tipo
  `HWND` y no compilar dos copias.
- **Frontend:** Svelte 5 + TypeScript sobre Vite, sin SvelteKit: es una sola ventana y
  el router no aporta nada. Las animaciones usan `Spring` de `svelte/motion`.
- **Separación de responsabilidades:** Rust es la fuente de verdad (providers de datos,
  ventana, hit-testing). El frontend solo dibuja el estado que recibe por eventos. Esto
  permitiría cambiar el render por uno nativo en el futuro sin tocar la lógica.

## Alternativas consideradas

| Opción                                     | Por qué no                                                                                                                                                                                         |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C# + WinUI 3                               | Las springs de Composition son excelentes, pero las ventanas sin marco con transparencia por píxel requieren hacks frágiles. Además requiere el SDK de .NET, que no está en el entorno del equipo. |
| C# + WPF                                   | `AllowsTransparency` usa ventanas _layered_ con copia a CPU en cada cuadro; las animaciones grandes se traban en monitores de 120–144 Hz.                                                          |
| Electron                                   | 150–300 MB de RAM para algo que vive encendido 24/7.                                                                                                                                               |
| Rust nativo (Direct2D / DirectComposition) | La mejor opción en rendimiento, pero obliga a construir un toolkit de UI propio: meses de trabajo antes de la primera actividad.                                                                   |

## Consecuencias

- **Positivas:** iteración de UI muy rápida (CSS, HMR de Vite), APIs de Windows completas
  desde Rust y un binario pequeño.
- **Negativas:** WebView2 suma memoria (procesos del navegador, ~80–150 MB en total).
  El proceso principal de la app mide ~30 MB.
- **Riesgo controlado:** `tao` (el gestor de ventanas de Tauri) es dueño de los estilos
  de la ventana y los reescribe en cada cambio de flags. Las reglas para convivir con eso
  están en el [ADR-0002](0002-ventana-fija-y-click-through.md).
