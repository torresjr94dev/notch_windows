//! Todo el FFI con Win32 vive aquí para aislar el `unsafe` del resto del código.
//!
//! Regla: las funciones que *modifican* la ventana se llaman en el hilo principal
//! (vía `AppHandle::run_on_main_thread`). tao guarda los estilos de la ventana en su
//! propio estado y los reaplica completos en cada cambio; tocarla desde otro hilo abre
//! carreras con él. Las de solo lectura sí se pueden llamar desde cualquier hilo.

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, GetWindowRect, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

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

/// Área del monitor principal (el que tiene su origen en 0,0).
pub fn primary_monitor_area() -> Option<MonitorArea> {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` vive en este stack frame y `cbSize` está inicializado como pide la API.
    let ok = unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    ok.then(|| MonitorArea {
        bounds: to_rect(info.rcMonitor),
        work: to_rect(info.rcWork),
    })
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

/// ¿Hay algo en pantalla completa? `QUNS_BUSY` cubre cualquier ventana a pantalla
/// completa (video, navegador con F11), `QUNS_RUNNING_D3D_FULL_SCREEN` los juegos en
/// modo exclusivo y `QUNS_PRESENTATION_MODE` el modo presentación de Windows.
pub fn fullscreen_app_running() -> bool {
    // SAFETY: la función no recibe argumentos.
    matches!(
        unsafe { SHQueryUserNotificationState() },
        Ok(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
    )
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
