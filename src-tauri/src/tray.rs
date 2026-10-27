//! Icono en la bandeja: la isla no tiene ventana principal ni aparece en la barra de
//! tareas, así que este es el único lugar para configurarla o cerrar la app.

use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::island::{IslandState, displays};
use crate::settings::{DisplayChoice, SettingsStore};

const TRAY_ID: &str = "main";
const TOGGLE_ID: &str = "toggle";
const QUIT_ID: &str = "quit";
const VISUALIZER_ID: &str = "visualizer";
const CONFIG_DIR_ID: &str = "config-dir";
const DISPLAY_PRIMARY_ID: &str = "display:primary";
const DISPLAY_ALL_ID: &str = "display:all";
/// Seguido del id estable del monitor.
const DISPLAY_MONITOR_PREFIX: &str = "display:monitor:";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Notch Windows")
        .menu(&build_menu(app)?)
        .on_menu_event(on_menu_event);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Reconstruye el menú: al conectar o desconectar monitores, o para reflejar un cambio
/// de preferencia. Rehacerlo completo es más simple y seguro que parchear sus items.
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    match build_menu(app) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(err) => eprintln!("no se pudo reconstruir el menú de la bandeja: {err}"),
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let state = app.state::<IslandState>();
    let settings = app.state::<SettingsStore>().get();
    let choice = settings.display;

    let toggle_label = if state.user_hidden() {
        "Mostrar isla"
    } else {
        "Ocultar isla"
    };
    let toggle = MenuItem::with_id(app, TOGGLE_ID, toggle_label, true, None::<&str>)?;

    let display_menu = Submenu::new(app, "Pantalla", true)?;
    display_menu.append(&CheckMenuItem::with_id(
        app,
        DISPLAY_PRIMARY_ID,
        "Principal de Windows",
        true,
        choice == DisplayChoice::Primary,
        None::<&str>,
    )?)?;
    display_menu.append(&PredefinedMenuItem::separator(app)?)?;
    let monitors = state.displays();
    for (display, label) in monitors.iter().zip(displays::labels(&monitors)) {
        let label = if display.primary {
            format!("{label} (principal)")
        } else {
            label
        };
        display_menu.append(&CheckMenuItem::with_id(
            app,
            format!("{DISPLAY_MONITOR_PREFIX}{}", display.id),
            label,
            true,
            choice == DisplayChoice::Monitor(display.id.clone()),
            None::<&str>,
        )?)?;
    }
    display_menu.append(&PredefinedMenuItem::separator(app)?)?;
    display_menu.append(&CheckMenuItem::with_id(
        app,
        DISPLAY_ALL_ID,
        "Todas las pantallas",
        true,
        choice == DisplayChoice::All,
        None::<&str>,
    )?)?;

    let visualizer = CheckMenuItem::with_id(
        app,
        VISUALIZER_ID,
        "Visualizador de audio",
        true,
        settings.visualizer,
        None::<&str>,
    )?;
    // Ahí viven settings.json y el token de la API que leen los scripts y hooks.
    let config = MenuItem::with_id(
        app,
        CONFIG_DIR_ID,
        "Abrir carpeta de configuración",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Salir", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[
            &toggle,
            &display_menu,
            &visualizer,
            &PredefinedMenuItem::separator(app)?,
            &config,
            &quit,
        ],
    )
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    let choice = match id {
        TOGGLE_ID => {
            app.state::<IslandState>().toggle_user_hidden();
            None
        }
        QUIT_ID => {
            app.exit(0);
            return;
        }
        VISUALIZER_ID => {
            app.state::<SettingsStore>()
                .update(|settings| settings.visualizer = !settings.visualizer);
            None
        }
        CONFIG_DIR_ID => {
            if let Ok(dir) = app.path().app_config_dir() {
                let _ = std::process::Command::new("explorer").arg(dir).spawn();
            }
            return;
        }
        DISPLAY_PRIMARY_ID => Some(DisplayChoice::Primary),
        DISPLAY_ALL_ID => Some(DisplayChoice::All),
        other => match other.strip_prefix(DISPLAY_MONITOR_PREFIX) {
            Some(monitor) => Some(DisplayChoice::Monitor(monitor.to_owned())),
            None => return,
        },
    };
    if let Some(choice) = choice {
        app.state::<SettingsStore>()
            .update(|settings| settings.display = choice);
        app.state::<IslandState>().request_reconcile();
    }
    // Windows marca/desmarca solo el item clickeado; el rebuild deja las palomitas
    // consistentes (un solo modo activo) y actualiza el texto de "Ocultar/Mostrar".
    refresh(app);
}
