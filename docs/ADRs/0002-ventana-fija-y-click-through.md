# ADR-0002: Ventana fija, click-through por polling de cursor y convivencia con tao

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

A diferencia del iPhone, en Windows no hay un hueco físico donde poner la isla: flota
**encima** de contenido real, como las pestañas del navegador o la barra de título de
las ventanas maximizadas. Por eso la ventana debe cumplir estas reglas:

- No bloquear clics fuera de la pastilla.
- No robar el foco nunca, ni al hacerle clic ni al mostrarse.
- No aparecer en la barra de tareas ni en Alt+Tab.
- Desaparecer cuando haya un juego, video o presentación en pantalla completa.
- Animar su tamaño de forma fluida.

## Decisiones

### 1. Ventana de tamaño fijo, la pastilla se anima dentro

La ventana mide 560×240 px lógicos y es transparente. La pastilla cambia de tamaño
**dentro** con CSS. Redimensionar una ventana nativa cuadro por cuadro se traba
(`SetWindowPos` más el re-layout de WebView2 en cada frame).

### 2. Click-through con hit-test en Rust

- El frontend reporta el área de la pastilla en píxeles físicos (`set_hit_rect`). Ese
  área se extiende hasta el borde superior de la ventana (ley de Fitts: el usuario avienta
  el mouse contra el tope de la pantalla).
- Un hilo vigilante revisa `GetCursorPos` cada 33 ms y alterna
  `set_ignore_cursor_events` (`WS_EX_TRANSPARENT`). Al salir de la pastilla aplica una
  histéresis de 8 px para evitar parpadeos en el borde.
- **Por qué polling y no un hook:** la alternativa por eventos es `WH_MOUSE_LL`, que se
  mete en el camino de todo el input del sistema. Si el hilo se atora, se atora el mouse de
  todo Windows. `GetCursorPos` a 30 Hz cuesta prácticamente cero.

### 3. Pantalla completa

Cada ~500 ms se consulta `SHQueryUserNotificationState`. Si responde `QUNS_BUSY`,
`QUNS_RUNNING_D3D_FULL_SCREEN` o `QUNS_PRESENTATION_MODE`, la isla se oculta. El ocultado
manual desde la bandeja es independiente: salir de pantalla completa no muestra de nuevo
una isla que el usuario ocultó.

### 4. Reglas para convivir con `tao` (verificado en tao 0.37)

`tao` guarda los flags de la ventana y, en **cada** cambio, reescribe `GWL_EXSTYLE`
completo y termina con `ShowWindow(SW_SHOW)` si cree que la ventana es visible. Las
consecuencias:

| Necesidad         | Lo que **no** funciona                                                      | Lo que hacemos                                                                                                               |
| ----------------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| No robar foco     | Poner `WS_EX_NOACTIVATE` a mano: se borra en el siguiente cambio de flags   | `"focusable": false` en `tauri.conf.json`; tao lo vuelve un flag persistente                                                 |
| Fuera del Alt+Tab | `skipTaskbar`: solo hace `ITaskbarList::DeleteTab` y deja `WS_EX_APPWINDOW` | Después de cada cambio de click-through se quita `WS_EX_APPWINDOW` y se pone `WS_EX_TOOLWINDOW` (`win32::hide_from_alt_tab`) |
| Mostrar sin foco  | `WebviewWindow::show()` usa `SW_SHOW`, que activa la ventana                | `ShowWindow(SW_SHOWNOACTIVATE)` / `SW_HIDE` directo (`win32::set_shown`)                                                     |

**Invariantes:**

1. Todo cambio a la ventana se ejecuta en el **hilo principal**
   (`AppHandle::run_on_main_thread`). Ahí tao aplica sus cambios de forma síncrona y
   podemos corregir los estilos justo después. Además, la cola es FIFO, así que el orden
   de las operaciones se respeta.
2. Mientras la isla está oculta **no se toca ningún flag de tao**, porque su
   `ShowWindow(SW_SHOW)` la volvería a mostrar y le daría el foco. Por eso, antes de
   ocultarla, se deja en click-through.

## Consecuencias

- Verificado en ejecución: la ventana nunca toma el foco, alterna `WS_EX_TRANSPARENT` al
  entrar y salir, y `WS_EX_APPWINDOW` no reaparece después de los cambios.
- Actualizar Tauri o tao exige revisar estas reglas: dependen de detalles de
  implementación de `tao::platform_impl::windows::window_state`.
- La isla solo vive en el monitor principal. Elegir otro monitor queda para la
  configuración de usuario.
