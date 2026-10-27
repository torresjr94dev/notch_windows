//! Hilo vigilante de las islas: decide en qué monitores hay isla, cuándo cada ventana
//! deja pasar los clics, la esconde si su monitor tiene algo en pantalla completa y la
//! mantiene anclada arriba al centro.
//!
//! Corre con polling a propósito. La alternativa por eventos para el cursor es un hook
//! global de mouse (`WH_MOUSE_LL`), que se mete en el camino de *todo* el input del
//! sistema: si este hilo se atora, se atora el mouse de todo Windows. `GetCursorPos`
//! a 30 Hz cuesta prácticamente cero y no pone nada en riesgo.
//!
//! Este hilo solo *lee* el estado de Windows. Todo cambio a una ventana se encola en el
//! hilo principal con `run_on_main_thread`, que respeta el orden de llegada.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use super::displays::{self, Action, Display, Fullscreen};
use super::geometry::{self, MonitorArea};
use super::win32::{self, RawHwnd};
use super::{IslandState, build_window, set_click_through};
use crate::settings::SettingsStore;

const TICK: Duration = Duration::from_millis(33);
/// Cada cuántos ticks se revisa lo que cambia lento: monitores y pantalla completa (~500 ms).
const SLOW_TICKS: u32 = 15;
/// Histéresis al salir de la pastilla, en píxeles físicos.
const HOVER_EXIT_MARGIN: i32 = 8;

pub const HOVER_EVENT: &str = "island:hover";

pub fn spawn(app: AppHandle) -> std::io::Result<()> {
    let watcher = Watcher {
        app,
        islands: Vec::new(),
        fullscreen: Fullscreen::None,
        next_label: 0,
    };
    thread::Builder::new()
        .name("island-watcher".into())
        .spawn(move || watcher.run())?;
    Ok(())
}

struct Island {
    label: String,
    display_id: String,
    /// `HMONITOR` de su pantalla, para cruzarlo con la detección de pantalla completa.
    monitor: isize,
    area: MonitorArea,
    window: WebviewWindow,
    hwnd: RawHwnd,
    hovered: bool,
    shown: bool,
}

struct Watcher {
    app: AppHandle,
    islands: Vec<Island>,
    fullscreen: Fullscreen,
    next_label: u32,
}

impl Watcher {
    fn run(mut self) -> ! {
        let mut tick: u32 = 0;
        loop {
            let reconcile_requested = self.app.state::<IslandState>().take_reconcile_request();
            if reconcile_requested || tick.is_multiple_of(SLOW_TICKS) {
                self.reconcile();
                self.fullscreen = win32::fullscreen_state();
            }
            tick = tick.wrapping_add(1);

            let user_hidden = self.app.state::<IslandState>().user_hidden();
            for index in 0..self.islands.len() {
                // Anclar en cada tick (no solo en el lento) porque al cruzar a un monitor
                // con otro DPI la ventana cambia de tamaño físico y hay que re-centrarla
                // de inmediato. Es un `GetWindowRect` y una comparación: no cuesta nada.
                self.anchor(index);
                self.update_visibility(index, user_hidden);
                if self.islands[index].shown {
                    self.update_hover(index);
                }
            }

            thread::sleep(TICK);
        }
    }

    /// Ajusta las islas al inventario de monitores y a la preferencia del usuario.
    fn reconcile(&mut self) {
        let displays = win32::enumerate_displays();
        // Handle propio para que el `State` no tenga prestado a `self` mientras se crean islas.
        let app = self.app.clone();
        let state = app.state::<IslandState>();
        if state.set_displays(displays.clone()) {
            crate::tray::refresh(&app);
        }

        let choice = app.state::<SettingsStore>().get().display;
        let targets = displays::targets(&choice, &displays);
        let existing: Vec<&str> = self.islands.iter().map(|i| i.display_id.as_str()).collect();
        let target_ids: Vec<&str> = targets.iter().map(|d| d.id.as_str()).collect();
        let actions = displays::plan(&existing, &target_ids);

        let find = |id: &str| displays.iter().find(|d| d.id == id);
        let mut removed = Vec::new();
        for action in actions {
            match action {
                Action::Reassign { index, display } => {
                    if let Some(d) = find(&display) {
                        self.islands[index].display_id = display;
                        self.islands[index].monitor = d.handle;
                        self.islands[index].area = d.area;
                    }
                }
                Action::Remove { index } => removed.push(index),
                Action::Create { display } => {
                    if let Some(island) = find(&display).and_then(|d| self.create(d)) {
                        self.islands.push(island);
                    }
                }
            }
        }
        // De mayor a menor para que los índices pendientes sigan siendo válidos.
        removed.sort_unstable_by(|a, b| b.cmp(a));
        for index in removed {
            let island = self.islands.remove(index);
            state.forget(&island.label);
            let _ = island.window.destroy();
        }

        // Los `HMONITOR` y las áreas cambian con la topología aunque el monitor sea el mismo.
        for island in &mut self.islands {
            if let Some(d) = find(&island.display_id) {
                island.monitor = d.handle;
                island.area = d.area;
            }
        }
    }

    fn create(&mut self, display: &Display) -> Option<Island> {
        let label = format!("island-{}", self.next_label);
        self.next_label += 1;
        let window = build_window(&self.app, &label)
            .inspect_err(|err| eprintln!("no se pudo crear la isla {label}: {err}"))
            .ok()?;
        let hwnd = RawHwnd::from(window.hwnd().ok()?);
        // Arranca transparente al mouse; `update_hover` la "prende" cuando el cursor entra.
        let w = window.clone();
        let _ = self
            .app
            .run_on_main_thread(move || set_click_through(&w, hwnd, true));
        Some(Island {
            label,
            display_id: display.id.clone(),
            monitor: display.handle,
            area: display.area,
            window,
            hwnd,
            hovered: false,
            shown: true,
        })
    }

    /// Reubica la ventana si no está arriba al centro de su monitor (cambió la
    /// resolución, el DPI, la barra de tareas o se reasignó a otra pantalla).
    fn anchor(&self, index: usize) {
        let island = &self.islands[index];
        let Some(current) = win32::window_rect(island.hwnd) else {
            return;
        };
        let (x, y) = geometry::island_origin(island.area, current.width);
        if (current.x, current.y) != (x, y) {
            let hwnd = island.hwnd;
            let _ = self
                .app
                .run_on_main_thread(move || win32::move_window(hwnd, x, y));
        }
    }

    fn update_visibility(&mut self, index: usize, user_hidden: bool) {
        let fullscreen_here = self.fullscreen.covers(self.islands[index].monitor);
        let island = &mut self.islands[index];
        let shown = should_show(user_hidden, fullscreen_here);
        if shown == island.shown {
            return;
        }
        island.shown = shown;

        let (window, hwnd) = (island.window.clone(), island.hwnd);
        let _ = self.app.run_on_main_thread(move || {
            if !shown {
                // Regresarla a click-through *antes* de esconderla: ya oculta no se le
                // puede tocar nada a tao (ver `win32::set_shown`).
                set_click_through(&window, hwnd, true);
            }
            win32::set_shown(hwnd, shown);
        });

        if !shown && island.hovered {
            island.hovered = false;
            let _ = self.app.emit_to(island.label.as_str(), HOVER_EVENT, false);
        }
    }

    fn update_hover(&mut self, index: usize) {
        let hit = self
            .app
            .state::<IslandState>()
            .hit_rect(&self.islands[index].label);
        let island = &mut self.islands[index];
        let margin = if island.hovered { HOVER_EXIT_MARGIN } else { 0 };
        let inside = cursor_inside(island.hwnd, hit, margin);
        if inside == island.hovered {
            return;
        }
        island.hovered = inside;

        // Fuera de la pastilla la ventana es "transparente" al mouse: los clics caen en
        // lo que haya debajo (p. ej. las pestañas de Chrome).
        let (window, hwnd) = (island.window.clone(), island.hwnd);
        let _ = self
            .app
            .run_on_main_thread(move || set_click_through(&window, hwnd, !inside));
        // Solo a su ventana: con un `emit` global el hover de un monitor abriría
        // las islas de todos.
        let _ = self.app.emit_to(island.label.as_str(), HOVER_EVENT, inside);
    }
}

/// Ambas razones para esconderla son independientes: salir de pantalla completa no
/// debe revivir una isla que el usuario ocultó a mano.
fn should_show(user_hidden: bool, fullscreen_here: bool) -> bool {
    !user_hidden && !fullscreen_here
}

fn cursor_inside(hwnd: RawHwnd, hit: geometry::Rect, margin: i32) -> bool {
    let (Some((cx, cy)), Some(win)) = (win32::cursor_pos(), win32::window_rect(hwnd)) else {
        return false;
    };
    hit.contains(cx - win.x, cy - win.y, margin)
}

#[cfg(test)]
mod tests {
    use super::should_show;

    #[test]
    fn shows_only_when_nothing_hides_it() {
        assert!(should_show(false, false));
        assert!(!should_show(true, false));
        assert!(!should_show(false, true));
        assert!(!should_show(true, true));
    }
}
