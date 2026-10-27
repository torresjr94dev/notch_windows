//! Hilo vigilante de la isla: decide cuándo la ventana deja pasar los clics, la esconde
//! si hay algo en pantalla completa y la mantiene anclada arriba al centro.
//!
//! Corre con polling a propósito. La alternativa por eventos para el cursor es un hook
//! global de mouse (`WH_MOUSE_LL`), que se mete en el camino de *todo* el input del
//! sistema: si este hilo se atora, se atora el mouse de todo Windows. `GetCursorPos`
//! a 30 Hz cuesta prácticamente cero y no pone nada en riesgo.
//!
//! Este hilo solo *lee* el estado de Windows. Todo cambio a la ventana se encola en el
//! hilo principal con `run_on_main_thread`, que respeta el orden de llegada.

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
    let watcher = Watcher {
        app,
        window,
        hwnd,
        hovered: false,
        shown: true,
        fullscreen: false,
    };
    thread::Builder::new()
        .name("island-watcher".into())
        .spawn(move || watcher.run())?;
    Ok(())
}

struct Watcher {
    app: AppHandle,
    window: WebviewWindow,
    hwnd: RawHwnd,
    hovered: bool,
    shown: bool,
    fullscreen: bool,
}

impl Watcher {
    fn run(mut self) -> ! {
        let mut tick: u32 = 0;
        loop {
            if tick.is_multiple_of(SLOW_TICKS) {
                self.keep_anchored();
                self.fullscreen = win32::fullscreen_app_running();
            }
            tick = tick.wrapping_add(1);

            self.update_visibility();
            if self.shown {
                self.update_hover();
            }

            thread::sleep(TICK);
        }
    }

    fn update_visibility(&mut self) {
        let shown = !self.fullscreen;
        if shown == self.shown {
            return;
        }
        self.shown = shown;

        let (window, hwnd) = (self.window.clone(), self.hwnd);
        let _ = self.app.run_on_main_thread(move || {
            if !shown {
                // Regresarla a click-through *antes* de esconderla: ya oculta no se le
                // puede tocar nada a tao (ver `win32::set_shown`).
                super::set_click_through(&window, hwnd, true);
            }
            win32::set_shown(hwnd, shown);
        });

        if !shown && self.hovered {
            self.hovered = false;
            let _ = self.app.emit(HOVER_EVENT, false);
        }
    }

    fn update_hover(&mut self) {
        let margin = if self.hovered { HOVER_EXIT_MARGIN } else { 0 };
        let hit = self.app.state::<IslandState>().hit_rect();
        let inside = cursor_inside(self.hwnd, hit, margin);
        if inside == self.hovered {
            return;
        }
        self.hovered = inside;

        // Fuera de la pastilla la ventana es "transparente" al mouse: los clics caen en
        // lo que haya debajo (p. ej. las pestañas de Chrome).
        let (window, hwnd) = (self.window.clone(), self.hwnd);
        let _ = self.app.run_on_main_thread(move || {
            super::set_click_through(&window, hwnd, !inside);
        });
        let _ = self.app.emit(HOVER_EVENT, inside);
    }

    /// Reubica la ventana si cambió el monitor principal, la resolución, el DPI (que
    /// cambia su tamaño físico) o la posición de la barra de tareas.
    fn keep_anchored(&self) {
        let (Some(area), Some(current)) =
            (win32::primary_monitor_area(), win32::window_rect(self.hwnd))
        else {
            return;
        };
        let (x, y) = geometry::island_origin(area, current.width);
        if (current.x, current.y) != (x, y) {
            let hwnd = self.hwnd;
            let _ = self
                .app
                .run_on_main_thread(move || win32::move_window(hwnd, x, y));
        }
    }
}

fn cursor_inside(hwnd: RawHwnd, hit: geometry::Rect, margin: i32) -> bool {
    let (Some((cx, cy)), Some(win)) = (win32::cursor_pos(), win32::window_rect(hwnd)) else {
        return false;
    };
    hit.contains(cx - win.x, cy - win.y, margin)
}
