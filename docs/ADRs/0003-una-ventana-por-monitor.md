# ADR-0003: Una ventana por monitor, creada en runtime

- **Fecha:** 2026-10-27
- **Estado:** Aceptado

## Contexto

El usuario debe poder elegir dónde vive la isla: en el monitor principal de Windows, en
un monitor concreto o en todos. Los monitores se conectan, se desconectan y cambian de
resolución o de DPI mientras la app corre.

## Decisiones

### Una ventana por monitor

Cada isla es una ventana propia (`island-0`, `island-1`, …) creada en runtime con
`WebviewWindowBuilder`; `tauri.conf.json` ya no declara ventanas. La alternativa, una
sola ventana que cubra todos los monitores, se descartó porque:

- Una ventana que cruza monitores con DPI distintos se renderiza con un solo factor de
  escala: la isla se vería borrosa o con el tamaño equivocado en alguno de ellos.
- Una ventana _topmost_ del tamaño del escritorio virtual interfiere con la detección de
  pantalla completa y con las optimizaciones de presentación de los juegos.

### El vigilante reconcilia

El hilo vigilante (ver ADR-0002) es el único que crea, mueve o destruye islas. Cada
~500 ms, o de inmediato si la bandeja lo pide, compara el inventario de monitores con la
preferencia y aplica un plan (`island::displays::plan`):

1. **Reasignar** una isla cuyo monitor ya no es destino a un destino nuevo. Reusar la
   ventana evita recargar el WebView y el parpadeo; por ejemplo, cuando cambia cuál es el
   monitor principal.
2. **Crear** islas para los destinos que sobren.
3. **Destruir** las islas que sobren.

El anclaje arriba al centro se revisa en **cada** tick (33 ms): al pasar a un monitor con
otro DPI, la ventana cambia de tamaño físico y hay que re-centrarla de inmediato.

### Identidad estable de los monitores

`\\.\DISPLAYn` no sirve como id porque Windows renumera al reconectar (en la máquina de
desarrollo ya va en `DISPLAY6`). El id es la ruta de dispositivo del monitor
(`monitorDevicePath` de `QueryDisplayConfig`), que incluye modelo y puerto. El nombre que
ve el usuario sale del EDID ("AW3425DWM", "DELL P2319H"). Si el monitor elegido no está
conectado, la isla se va al principal sin tocar la preferencia, y regresa sola cuando se
reconecte.

### Eventos dirigidos

Un `listen` global en JS recibe **todos** los eventos, aunque vayan dirigidos a otra
ventana (en Tauri 2.12, `EventTarget::Any` siempre hace match). Por eso:

- Los eventos por isla (hover) se emiten con `emit_to(label)` y el frontend los escucha
  con `getCurrentWebviewWindow().listen`.
- Los eventos para todas (el estado de las actividades) se emiten con `emit` y se
  escuchan con `listen`.

### Pantalla completa por monitor

`SHQueryUserNotificationState` es global. Para no esconder la isla del monitor 1 cuando
hay un juego en el monitor 2, `QUNS_BUSY` y `QUNS_RUNNING_D3D_FULL_SCREEN` se atribuyen al
monitor de la ventana en primer plano (`MonitorFromWindow`). Solo
`QUNS_PRESENTATION_MODE` oculta las islas de todas las pantallas.

## Consecuencias

- Con "todas las pantallas" hay un WebView por monitor. Comparten el proceso del
  navegador, pero cada uno suma memoria (~20–30 MB).
- Las preferencias viven en `%APPDATA%\dev.notchwindows.island\settings.json`. Se
  escriben de forma atómica (archivo temporal + rename) y un archivo corrupto no impide
  arrancar: se usan los valores por defecto.
