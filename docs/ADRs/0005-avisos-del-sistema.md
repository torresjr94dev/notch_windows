# ADR-0005: Avisos del sistema: fuentes, eventos vs. polling y el OSD nativo

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

La isla avisa cambios de volumen, brillo, batería y Bloq Mayús, y muestra qué apps usan
el micrófono o la cámara. Cada fuente tiene su propia API en Windows, con costos y
riesgos distintos. La app vive encendida todo el día, así que importa cuánto consume en
reposo y cuánto riesgo mete en el input del sistema.

## Decisión: una fuente por aviso

| Aviso      | Fuente                                                                             | Cómo se entera                |
| ---------- | ---------------------------------------------------------------------------------- | ----------------------------- |
| Volumen    | `IAudioEndpointVolume` de la salida por defecto                                    | Callback COM (`#[implement]`) |
| Brillo     | WMI `WmiMonitorBrightnessEvent` (solo paneles integrados)                          | Notificación WMI bloqueante   |
| Batería    | `GetSystemPowerStatus`                                                             | Polling cada 1 s              |
| Bloq Mayús | `GetKeyState(VK_CAPITAL)`                                                          | Polling cada 50 ms            |
| Micrófono  | Sesiones de audio activas en los dispositivos de captura (`IAudioSessionManager2`) | Polling cada 1 s              |
| Cámara     | `IMFSensorActivityMonitor` de Media Foundation                                     | Callback COM con cada reporte |

### Por qué hay polling en tres casos

- **Bloq Mayús:** la alternativa por eventos es un hook global de teclado
  (`WH_KEYBOARD_LL`), que ve _cada tecla_ del sistema. Si el hilo se atora, se atora el
  teclado de todo Windows, y además lo marcan los antivirus. Leer una sola tecla de
  estado cada 50 ms cuesta prácticamente nada. Se verificó en ejecución que
  `GetKeyState` refleja el estado real desde un hilo sin ventana ni foco.
- **Batería:** el evento (`WM_POWERBROADCAST`) llega a una ventana, y nuestras ventanas
  son de tao. Una lectura por segundo de una estructura en memoria es más simple que
  crear una ventana oculta solo para eso.
- **Micrófono:** Core Audio avisa altas de sesiones (`IAudioSessionNotification`), pero
  no cambios de estado sin suscribirse a cada sesión por separado. Enumerar las sesiones
  de captura es una consulta en memoria; hacerlo cada segundo es más simple y no se nota.

### Privacidad: indicador, no actividad

El uso de micrófono o cámara no compite por la pastilla: viaja en la vista
(`IslandView.privacy`) y se dibuja como una burbuja a la izquierda, junto a cualquier
actividad. Ninguna de las dos fuentes pide permisos ni abre los dispositivos: solo
observa quién los está usando. El nombre de la app sale del ejecutable del proceso.

### Enmienda (2026-10-27): el registro `ConsentStore` ya no sirve

La primera versión leía `HKCU\…\CapabilityAccessManager\ConsentStore` (una app usa el
dispositivo mientras su `LastUsedTimeStop` vale 0). En la build 26300 de Windows 11 eso
**no se actualiza**: con la cámara y el micrófono en uso, las entradas más recientes de
HKCU y HKLM tenían semanas. Windows ya no lleva ese registro al día, así que la burbuja
nunca aparecía. Se cambió a las dos APIs documentadas de la tabla, que reportan el uso en
vivo y no dependen de detalles internos del sistema.

## El OSD nativo de Windows se queda

Al cambiar el volumen o el brillo, Windows muestra su propio OSD además de la isla.
**No se oculta a propósito:**

- No hay API oficial para ocultarlo.
- Los hacks conocidos (buscar la ventana del OSD y minimizarla, inyectarse en el shell)
  dependen de nombres de clase internos que cambian entre builds de Windows 11, y en
  24H2 el OSD es parte del Shell Experience Host.
- Un hack roto deja al usuario **sin** OSD de volumen, que es peor que tener dos.

Si algún día existe una API (o una opción del sistema), se reevalúa.

## Consecuencias

- En una PC de escritorio el brillo y la batería no publican nada. Sus providers
  terminan solos y no consumen recursos.
- Los nombres de apps en privacidad salen del ejecutable del proceso. Hay un mapa corto
  de nombres conocidos (Teams, OBS, Cámara…); lo demás se muestra capitalizado tal cual,
  y un proceso que no se deja inspeccionar (elevado o del sistema) aparece como "Otra
  app": el indicador nunca se calla por no saber el nombre.
