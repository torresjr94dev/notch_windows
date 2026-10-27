//! La isla: ventana transparente, siempre encima y que nunca roba el foco, anclada
//! arriba al centro del monitor principal.
//!
//! La ventana es de tamaño fijo y la pastilla se anima *dentro* de ella con CSS:
//! redimensionar una ventana nativa cuadro por cuadro se traba. Ver
//! `docs/ADRs/0002-ventana-fija-y-click-through.md`.

mod geometry;
mod watcher;
mod win32;

use tauri::{AppHandle, WebviewWindow};

pub fn start(app: AppHandle, window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = win32::RawHwnd::from(window.hwnd()?);
    watcher::spawn(app, hwnd)?;
    Ok(())
}
