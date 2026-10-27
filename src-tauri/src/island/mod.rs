//! La isla: ventana transparente, siempre encima y que nunca roba el foco, anclada
//! arriba al centro del monitor principal.
//!
//! La ventana es de tamaño fijo y la pastilla se anima *dentro* de ella con CSS:
//! redimensionar una ventana nativa cuadro por cuadro se traba. Por eso la ventana deja
//! pasar los clics en todo lo que no sea la pastilla. Ver
//! `docs/ADRs/0002-ventana-fija-y-click-through.md`.

mod geometry;
mod watcher;
mod win32;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, State, WebviewWindow};

use geometry::Rect;

/// Estado compartido entre los comandos del frontend, la bandeja y el hilo vigilante.
#[derive(Default)]
pub struct IslandState {
    /// Área sensible de la pastilla, en píxeles físicos relativos a la ventana.
    hit_rect: Mutex<Rect>,
    /// El usuario la ocultó desde la bandeja.
    user_hidden: AtomicBool,
}

impl IslandState {
    fn hit_rect(&self) -> Rect {
        // `Rect` es `Copy`: aunque otro hilo haya hecho panic con el lock tomado,
        // el valor guardado sigue siendo válido.
        *self.hit_rect.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn user_hidden(&self) -> bool {
        self.user_hidden.load(Ordering::Relaxed)
    }

    /// Alterna el ocultado manual y devuelve el valor nuevo. Solo cambia la bandera:
    /// el vigilante es el único que muestra u oculta la ventana.
    pub fn toggle_user_hidden(&self) -> bool {
        !self.user_hidden.fetch_xor(true, Ordering::Relaxed)
    }
}

/// El frontend reporta aquí el área de la pastilla cada vez que cambia de tamaño.
#[tauri::command]
pub fn set_hit_rect(rect: Rect, state: State<'_, IslandState>) {
    *state
        .hit_rect
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = rect;
}

/// Prende o apaga el click-through. Llamar en el hilo principal: ahí tao aplica el
/// cambio de forma síncrona y podemos corregir sus estilos justo después.
fn set_click_through(window: &WebviewWindow, hwnd: win32::RawHwnd, enabled: bool) {
    let _ = window.set_ignore_cursor_events(enabled);
    win32::hide_from_alt_tab(hwnd);
}

/// Se llama desde `setup`, que corre en el hilo principal.
pub fn start(app: AppHandle, window: WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = win32::RawHwnd::from(window.hwnd()?);
    // Arranca transparente al mouse; el vigilante la "prende" cuando el cursor entra.
    set_click_through(&window, hwnd, true);
    watcher::spawn(app, window, hwnd)?;
    Ok(())
}
