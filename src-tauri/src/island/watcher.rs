//! Hilo vigilante de la isla: la mantiene anclada arriba al centro del monitor principal.
//!
//! Corre con polling a propósito: lo que vigila (monitor, resolución, DPI, barra de
//! tareas) no tiene un evento único y barato al que suscribirse desde Tauri, y revisarlo
//! cada medio segundo cuesta prácticamente cero.

use std::thread;
use std::time::Duration;

use tauri::AppHandle;

use super::geometry;
use super::win32::{self, RawHwnd};

const TICK: Duration = Duration::from_millis(33);
/// Cada cuántos ticks se revisa lo que cambia lento (~500 ms).
const SLOW_TICKS: u32 = 15;

pub fn spawn(app: AppHandle, hwnd: RawHwnd) -> std::io::Result<()> {
    thread::Builder::new()
        .name("island-watcher".into())
        .spawn(move || {
            let mut tick: u32 = 0;
            loop {
                if tick.is_multiple_of(SLOW_TICKS) {
                    keep_anchored(&app, hwnd);
                }
                tick = tick.wrapping_add(1);
                thread::sleep(TICK);
            }
        })?;
    Ok(())
}

/// Reubica la ventana si cambió el monitor principal, la resolución, el DPI (que
/// cambia su tamaño físico) o la posición de la barra de tareas.
fn keep_anchored(app: &AppHandle, hwnd: RawHwnd) {
    let (Some(area), Some(current)) = (win32::primary_monitor_area(), win32::window_rect(hwnd))
    else {
        return;
    };
    let (x, y) = geometry::island_origin(area, current.width);
    if (current.x, current.y) != (x, y) {
        let _ = app.run_on_main_thread(move || win32::move_window(hwnd, x, y));
    }
}
