mod activity;
mod api;
mod autostart;
mod focus;
mod island;
mod providers;
mod settings;
mod tray;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();
    // Una sola isla por sesión: con "Iniciar con Windows" y el acceso directo de Inicio
    // es fácil abrir dos, encimadas y peleando por el puerto de la API. Abrirla otra vez
    // muestra la que ya corre si estaba oculta. Solo en release: en desarrollo, `tauri
    // dev` se cerraría en silencio con la versión instalada abierta (mismo identifier).
    if !cfg!(debug_assertions) {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if app.state::<island::IslandState>().unhide() {
                tray::refresh(app);
            }
        }));
    }
    builder
        .manage(island::IslandState::default())
        .invoke_handler(tauri::generate_handler![
            island::set_hit_rect,
            activity::get_view,
            activity::focus_activity,
            providers::media::media_control,
            providers::media::get_artwork,
            providers::volume::adjust_volume,
        ])
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let settings = settings::SettingsStore::load(config_dir.join("settings.json"));
            let api_port = settings.get().api_port;
            app.manage(settings);

            let hub = activity::start(app.handle())?;
            let media = providers::media::start(hub.clone())?;
            providers::spectrum::spawn(app.handle().clone(), media.playing.clone())?;
            app.manage(providers::volume::spawn(hub.clone())?);
            providers::brightness::spawn(hub.clone())?;
            providers::power::spawn(hub.clone())?;
            providers::keyboard::spawn(hub.clone())?;
            providers::privacy::spawn(hub.clone())?;
            let token = api::token::load_or_create(&config_dir.join("api-token"))?;
            api::start(hub.clone(), api_port, token)?;
            app.manage(media);
            app.manage(hub);

            island::start(app.handle().clone(), &app.state::<island::IslandState>())?;
            tray::create(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
