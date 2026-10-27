//! Hilo vigilante de la isla: decide cuándo la ventana deja pasar los clics y la
//! mantiene anclada arriba al centro del monitor principal.
//!
//! Corre con polling a propósito. La alternativa por eventos para el cursor es un hook
//! global de mouse (`WH_MOUSE_LL`), que se mete en el camino de *todo* el input del
//! sistema: si este hilo se atora, se atora el mouse de todo Windows. `GetCursorPos`
//! a 30 Hz cuesta prácticamente cero y no pone nada en riesgo.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use super::IslandState;
use super::geometry;
use super::win32::{self, RawHwnd};

const TICK: Duration = Duration::from_millis(33);
/// Cada cuántos ticks se revisa lo que cambia lento (~500 ms).
const SLOW_TICKS: u32 = 15;
/// Histéresis al salir de la pastilla, en píxeles físicos.
const HOVER_EXIT_MARGIN: i32 = 8;

pub const HOVER_EVENT: &str = "island:hover";

pub fn spawn(app: AppHandle, window: WebviewWindow, hwnd: RawHwnd) -> std::io::Result<()> {
    thread::Builder::new()
        .name("island-watcher".into())
        .spawn(move || {
            let state = app.state::<IslandState>();
            let mut hovered = false;
            let mut tick: u32 = 0;
            loop {
                if tick.is_multiple_of(SLOW_TICKS) {
                    keep_anchored(&app, hwnd);
                }
                tick = tick.wrapping_add(1);

                let margin = if hovered { HOVER_EXIT_MARGIN } else { 0 };
                let inside = cursor_inside(hwnd, state.hit_rect(), margin);
                if inside != hovered {
                    hovered = inside;
                    // Fuera de la pastilla la ventana es "transparente" al mouse: los
                    // clics caen en lo que haya debajo (p. ej. las pestañas de Chrome).
                    let window = window.clone();
                    let _ = app.run_on_main_thread(move || {
                        super::set_click_through(&window, hwnd, !inside);
                    });
                    let _ = app.emit(HOVER_EVENT, inside);
                }

                thread::sleep(TICK);
            }
        })?;
    Ok(())
}

fn cursor_inside(hwnd: RawHwnd, hit: geometry::Rect, margin: i32) -> bool {
    let (Some((cx, cy)), Some(win)) = (win32::cursor_pos(), win32::window_rect(hwnd)) else {
        return false;
    };
    hit.contains(cx - win.x, cy - win.y, margin)
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
