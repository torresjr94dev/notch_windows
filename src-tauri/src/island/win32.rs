//! Todo el FFI con Win32 vive aquí para aislar el `unsafe` del resto del código.
//!
//! Regla: las funciones que *modifican* la ventana se llaman en el hilo principal
//! (vía `AppHandle::run_on_main_thread`). tao guarda los estilos de la ventana en su
//! propio estado y los reaplica completos en cada cambio; tocarla desde otro hilo abre
//! carreras con él. Las de solo lectura sí se pueden llamar desde cualquier hilo.

use std::collections::HashMap;
use std::ffi::c_void;

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
    DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
    QueryDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MONITORINFOEXW, MonitorFromWindow,
};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    MONITORINFOF_PRIMARY, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, ShowWindow, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW,
};
use windows::core::BOOL;

use super::displays::{Display, Fullscreen};
use super::geometry::{MonitorArea, Rect};

/// `HWND` envuelve un puntero crudo y por eso no es `Send`. Guardamos su valor
/// numérico para poder pasarlo al hilo del vigilante.
#[derive(Clone, Copy, Debug)]
pub struct RawHwnd(isize);

impl From<HWND> for RawHwnd {
    fn from(hwnd: HWND) -> Self {
        Self(hwnd.0 as isize)
    }
}

impl RawHwnd {
    fn get(self) -> HWND {
        HWND(self.0 as *mut c_void)
    }
}

fn to_rect(r: RECT) -> Rect {
    Rect {
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
    }
}

fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Monitores conectados, ordenados de izquierda a derecha para que el menú siga el
/// acomodo físico.
pub fn enumerate_displays() -> Vec<Display> {
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        // SAFETY: `data` es el `&mut Vec` que pasamos abajo y vive durante toda la
        // llamada síncrona a `EnumDisplayMonitors`.
        unsafe { (*(data.0 as *mut Vec<HMONITOR>)).push(monitor) };
        true.into()
    }

    let mut monitors: Vec<HMONITOR> = Vec::new();
    // SAFETY: ver el comentario del callback.
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut monitors as *mut Vec<HMONITOR> as isize),
        )
    };

    let names = display_config_names();
    let mut displays: Vec<Display> = monitors
        .into_iter()
        .filter_map(|monitor| {
            let mut info = MONITORINFOEXW {
                monitorInfo: MONITORINFO {
                    cbSize: size_of::<MONITORINFOEXW>() as u32,
                    ..Default::default()
                },
                ..Default::default()
            };
            // SAFETY: `cbSize` declara la variante extendida, que es la que pasamos.
            let ok = unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.as_bool();
            ok.then(|| {
                let gdi_name = from_wide(&info.szDevice);
                let (id, name, internal) =
                    names
                        .get(&gdi_name)
                        .cloned()
                        .unwrap_or((gdi_name, String::new(), false));
                Display {
                    id,
                    name,
                    internal,
                    handle: monitor.0 as isize,
                    area: MonitorArea {
                        bounds: to_rect(info.monitorInfo.rcMonitor),
                        work: to_rect(info.monitorInfo.rcWork),
                    },
                    primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                }
            })
        })
        .collect();
    displays.sort_by_key(|d| (d.area.bounds.x, d.area.bounds.y));
    displays
}

/// Nombre GDI (`\\.\DISPLAY1`) → (id estable, nombre EDID, ¿panel integrado?).
///
/// GDI solo conoce `\\.\DISPLAYn`; el modelo y la ruta estable del monitor vienen de la
/// API de configuración de pantallas (CCD), que se cruza con GDI por ese nombre.
fn display_config_names() -> HashMap<String, (String, String, bool)> {
    let mut names = HashMap::new();
    let Some(paths) = active_display_paths() else {
        return names;
    };
    for path in paths {
        let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            },
            ..Default::default()
        };
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        // SAFETY: cada header declara el tamaño real de la estructura que lo contiene.
        let ok = unsafe {
            DisplayConfigGetDeviceInfo(&mut source.header) == 0
                && DisplayConfigGetDeviceInfo(&mut target.header) == 0
        };
        if !ok {
            continue;
        }
        let gdi_name = from_wide(&source.viewGdiDeviceName);
        let device_path = from_wide(&target.monitorDevicePath);
        let internal = matches!(
            target.outputTechnology,
            DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL
                | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED
                | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED
                | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_LVDS
        );
        let id = if device_path.is_empty() {
            gdi_name.clone()
        } else {
            device_path
        };
        // En modo "duplicar" una fuente tiene varios destinos: basta con el primero.
        names.entry(gdi_name).or_insert((
            id,
            from_wide(&target.monitorFriendlyDeviceName),
            internal,
        ));
    }
    names
}

fn active_display_paths() -> Option<Vec<DISPLAYCONFIG_PATH_INFO>> {
    // La topología puede cambiar entre pedir los tamaños y leer los datos
    // (ERROR_INSUFFICIENT_BUFFER); en ese caso se reintenta.
    for _ in 0..3 {
        let (mut path_count, mut mode_count) = (0u32, 0u32);
        // SAFETY: out-params válidos; los buffers se dimensionan con lo que reporta la API.
        unsafe {
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
                != ERROR_SUCCESS
            {
                return None;
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
            if QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            ) == ERROR_SUCCESS
            {
                paths.truncate(path_count as usize);
                return Some(paths);
            }
        }
    }
    None
}

/// Posición del cursor en coordenadas de pantalla (píxeles físicos: tao declara al
/// proceso como per-monitor DPI aware).
pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut point = POINT::default();
    // SAFETY: `point` es un out-param válido.
    unsafe { GetCursorPos(&mut point) }
        .ok()
        .map(|_| (point.x, point.y))
}

/// Rectángulo de la ventana en coordenadas de pantalla. Como la isla no tiene marco
/// ni sombra, coincide con el área cliente.
pub fn window_rect(hwnd: RawHwnd) -> Option<Rect> {
    let mut rect = RECT::default();
    // SAFETY: `rect` es un out-param válido; un HWND ya destruido solo hace fallar la llamada.
    unsafe { GetWindowRect(hwnd.get(), &mut rect) }
        .ok()
        .map(|_| to_rect(rect))
}

/// Saca la ventana del Alt+Tab.
///
/// El `skipTaskbar` de tao solo quita la pestaña de la barra de tareas
/// (`ITaskbarList::DeleteTab`) pero deja `WS_EX_APPWINDOW`, que obliga a Windows a
/// listar la ventana en Alt+Tab. Como tao reescribe `GWL_EXSTYLE` completo en cada
/// cambio de flags, hay que reaplicar esto después de cada uno. Llamar en el hilo principal.
pub fn hide_from_alt_tab(hwnd: RawHwnd) {
    let hwnd = hwnd.get();
    // SAFETY: sin punteros de por medio; un HWND inválido solo hace fallar las llamadas.
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let fixed = (current & !(WS_EX_APPWINDOW.0 as isize)) | WS_EX_TOOLWINDOW.0 as isize;
        if fixed != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, fixed);
            // Los cambios de estilo no se aplican hasta forzar un recálculo del marco.
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

/// ¿Dónde hay algo en pantalla completa? `QUNS_BUSY` cubre cualquier ventana a pantalla
/// completa (video, navegador con F11) y `QUNS_RUNNING_D3D_FULL_SCREEN` los juegos en
/// modo exclusivo; en ambos casos es la ventana en primer plano, así que solo se tapa
/// su monitor. `QUNS_PRESENTATION_MODE` es el modo presentación de Windows y aplica a
/// todas las pantallas.
pub fn fullscreen_state() -> Fullscreen {
    // SAFETY: ninguna de las llamadas recibe punteros.
    unsafe {
        match SHQueryUserNotificationState() {
            Ok(QUNS_PRESENTATION_MODE) => Fullscreen::Everywhere,
            Ok(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN) => {
                let foreground = GetForegroundWindow();
                if foreground.is_invalid() {
                    Fullscreen::Everywhere
                } else {
                    let monitor = MonitorFromWindow(foreground, MONITOR_DEFAULTTONEAREST);
                    Fullscreen::Monitor(monitor.0 as isize)
                }
            }
            _ => Fullscreen::None,
        }
    }
}

/// Muestra u oculta la ventana sin activarla. Llamar en el hilo principal.
///
/// No usamos `WebviewWindow::show()` porque tao muestra con `SW_SHOW`, que activa la
/// ventana y le roba el foco al juego del que el usuario acaba de salir.
///
/// Ojo: tao no se entera de esto y sigue creyendo que la ventana es visible. Cualquier
/// cambio de flags de tao (p. ej. `set_ignore_cursor_events`) termina en un
/// `ShowWindow(SW_SHOW)`, así que mientras esté oculta no se le debe tocar nada.
pub fn set_shown(hwnd: RawHwnd, shown: bool) {
    let cmd = if shown { SW_SHOWNOACTIVATE } else { SW_HIDE };
    // SAFETY: sin punteros de por medio; un HWND inválido solo hace fallar la llamada.
    let _ = unsafe { ShowWindow(hwnd.get(), cmd) };
}

/// Mueve la ventana sin redimensionarla, sin tocar su orden Z y sin activarla.
/// Llamar en el hilo principal.
pub fn move_window(hwnd: RawHwnd, x: i32, y: i32) {
    // SAFETY: sin punteros de por medio; un HWND inválido solo hace fallar la llamada.
    let _ = unsafe {
        SetWindowPos(
            hwnd.get(),
            None,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}
