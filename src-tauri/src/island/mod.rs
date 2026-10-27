//! Las islas: una ventana transparente por monitor elegido, siempre encima, que nunca
//! roba el foco y vive anclada arriba al centro de su pantalla.
//!
//! Cada ventana es de tamaño fijo y la pastilla se anima *dentro* de ella con CSS:
//! redimensionar una ventana nativa cuadro por cuadro se traba. Por eso la ventana deja
//! pasar los clics en todo lo que no sea la pastilla. Ver
//! `docs/ADRs/0002-ventana-fija-y-click-through.md` y `docs/ADRs/0003-una-ventana-por-monitor.md`.

pub mod displays;
mod geometry;
mod watcher;
mod win32;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::{AppHandle, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use displays::Display;
use geometry::Rect;

/// Tamaño lógico de cada ventana: cabe la isla expandida más el rebote del spring y su sombra.
const WINDOW_SIZE: (f64, f64) = (560.0, 240.0);

/// Estado compartido entre los comandos del frontend, la bandeja y el hilo vigilante.
#[derive(Default)]
pub struct IslandState {
    /// Área sensible de cada pastilla (por label de ventana), en píxeles físicos
    /// relativos a su ventana.
    hit_rects: Mutex<HashMap<String, Rect>>,
    /// Último inventario de monitores, para que la bandeja arme su menú.
    displays: Mutex<Vec<Display>>,
    /// El usuario la ocultó desde la bandeja.
    user_hidden: AtomicBool,
    /// Pide al vigilante reacomodar las islas ya, sin esperar su siguiente revisión.
    reconcile_requested: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Lo que guardan estos mutex son copias de datos: aunque otro hilo haya hecho panic
    // con el lock tomado, el valor sigue siendo válido.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl IslandState {
    fn hit_rect(&self, label: &str) -> Rect {
        lock(&self.hit_rects)
            .get(label)
            .copied()
            .unwrap_or_default()
    }

    fn forget(&self, label: &str) {
        lock(&self.hit_rects).remove(label);
    }

    pub fn displays(&self) -> Vec<Display> {
        lock(&self.displays).clone()
    }

    /// Guarda el inventario y dice si cambió.
    fn set_displays(&self, displays: Vec<Display>) -> bool {
        let mut current = lock(&self.displays);
        let changed = *current != displays;
        *current = displays;
        changed
    }

    pub fn user_hidden(&self) -> bool {
        self.user_hidden.load(Ordering::Relaxed)
    }

    /// Alterna el ocultado manual y devuelve el valor nuevo. Solo cambia la bandera:
    /// el vigilante es el único que muestra u oculta las ventanas.
    pub fn toggle_user_hidden(&self) -> bool {
        !self.user_hidden.fetch_xor(true, Ordering::Relaxed)
    }

    /// Quita el ocultado manual y dice si estaba oculta.
    pub fn unhide(&self) -> bool {
        self.user_hidden.swap(false, Ordering::Relaxed)
    }

    pub fn request_reconcile(&self) {
        self.reconcile_requested.store(true, Ordering::Relaxed);
    }

    fn take_reconcile_request(&self) -> bool {
        self.reconcile_requested.swap(false, Ordering::Relaxed)
    }
}

/// El frontend reporta aquí el área de su pastilla cada vez que cambia de tamaño.
#[tauri::command]
pub fn set_hit_rect(window: WebviewWindow, rect: Rect, state: State<'_, IslandState>) {
    lock(&state.hit_rects).insert(window.label().to_owned(), rect);
}

/// Prende o apaga el click-through. Llamar en el hilo principal: ahí tao aplica el
/// cambio de forma síncrona y podemos corregir sus estilos justo después.
fn set_click_through(window: &WebviewWindow, hwnd: win32::RawHwnd, enabled: bool) {
    let _ = window.set_ignore_cursor_events(enabled);
    win32::hide_from_alt_tab(hwnd);
}

/// Crea la ventana de una isla. Nace visible pero sin activarse (`focused(false)` hace
/// que tao la muestre con `SW_SHOWNOACTIVATE`): crearla oculta y mostrarla después
/// pasaría por el `SW_SHOW` de tao, que roba el foco. Mientras el WebView carga, la
/// ventana es transparente, así que no se nota que nace en otra posición.
///
/// No llamar en el hilo principal: `build` espera a que ese hilo cree la ventana.
fn build_window(app: &AppHandle, label: &str) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title("Notch Windows")
        .inner_size(WINDOW_SIZE.0, WINDOW_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .build()
}

/// Toma un inventario inicial (para que la bandeja nazca con su menú completo) y
/// arranca el vigilante, que es quien crea las ventanas.
pub fn start(app: AppHandle, state: &IslandState) -> std::io::Result<()> {
    state.set_displays(win32::enumerate_displays());
    watcher::spawn(app)
}
