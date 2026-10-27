# ADR-0005: Avisos del sistema: fuentes, eventos vs. polling y el OSD nativo

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

La isla avisa cambios de volumen, brillo, batería y Bloq Mayús, y muestra qué apps usan
el micrófono o la cámara. Cada fuente tiene su propia API en Windows, con costos y
riesgos distintos. La app vive encendida todo el día, así que importa cuánto consume en
reposo y cuánto riesgo mete en el input del sistema.

## Decisión: una fuente por aviso

| Aviso        | Fuente                                                                             | Cómo se entera                |
| ------------ | ---------------------------------------------------------------------------------- | ----------------------------- |
| Volumen      | `IAudioEndpointVolume` de la salida por defecto                                    | Callback COM (`#[implement]`) |
| Brillo       | WMI `WmiMonitorBrightnessEvent` (solo paneles integrados)                          | Notificación WMI bloqueante   |
| Batería      | `GetSystemPowerStatus`                                                             | Polling cada 1 s              |
| Bloq Mayús   | `GetKeyState(VK_CAPITAL)`                                                          | Polling cada 50 ms            |
| Mic / cámara | Registro `CapabilityAccessManager\ConsentStore` (`LastUsedTimeStop == 0` = en uso) | `RegNotifyChangeKeyValue`     |

### Por qué hay polling en dos casos

- **Bloq Mayús:** la alternativa por eventos es un hook global de teclado
  (`WH_KEYBOARD_LL`), que ve _cada tecla_ del sistema. Si el hilo se atora, se atora el
  teclado de todo Windows, y además lo marcan los antivirus. Leer una sola tecla de
  estado cada 50 ms cuesta prácticamente nada. Se verificó en ejecución que
  `GetKeyState` refleja el estado real desde un hilo sin ventana ni foco.
- **Batería:** el evento (`WM_POWERBROADCAST`) llega a una ventana, y nuestras ventanas
  son de tao. Una lectura por segundo de una estructura en memoria es más simple que
  crear una ventana oculta solo para eso.

### Privacidad: indicador, no actividad

El uso de micrófono o cámara no compite por la pastilla: viaja en la vista
(`IslandView.privacy`) y se dibuja como una burbuja a la izquierda, junto a cualquier
actividad. La fuente es el mismo registro que usa el ícono de privacidad de la barra de
tareas, así que no hace falta pedir permisos ni tocar los dispositivos.

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
- Los nombres de apps en privacidad salen de la subclave del registro (ejecutable o
  paquete). Hay un mapa corto de nombres conocidos (Teams, OBS…); lo demás se muestra
  capitalizado tal cual.
