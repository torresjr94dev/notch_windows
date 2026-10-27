//! Icono en la bandeja: la isla no tiene ventana principal ni aparece en la barra de
//! tareas, así que este es el único lugar para ocultarla o cerrar la app.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::island::IslandState;

const TOGGLE_ID: &str = "toggle";
const QUIT_ID: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, TOGGLE_ID, "Ocultar isla", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Notch Windows")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            TOGGLE_ID => {
                let hidden = app.state::<IslandState>().toggle_user_hidden();
                let label = if hidden {
                    "Mostrar isla"
                } else {
                    "Ocultar isla"
                };
                let _ = toggle.set_text(label);
            }
            QUIT_ID => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
