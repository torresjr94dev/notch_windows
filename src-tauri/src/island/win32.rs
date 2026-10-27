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
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
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

/// Rectángulo de la ventana en coordenadas de pantalla. Como la isla no tiene marco
/// ni sombra, coincide con el área cliente.
pub fn window_rect(hwnd: RawHwnd) -> Option<Rect> {
    let mut rect = RECT::default();
    // SAFETY: `rect` es un out-param válido; un HWND ya destruido solo hace fallar la llamada.
    unsafe { GetWindowRect(hwnd.get(), &mut rect) }
        .ok()
        .map(|_| to_rect(rect))
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
