mod activity;
mod island;
mod providers;
mod settings;
mod tray;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(island::IslandState::default())
        .invoke_handler(tauri::generate_handler![
            island::set_hit_rect,
            activity::get_view,
            providers::media::media_control,
            providers::media::get_artwork,
        ])
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            app.manage(settings::SettingsStore::load(settings_path));

            let hub = activity::start(app.handle())?;
            app.manage(providers::media::start(hub.clone())?);
            app.manage(hub);

            island::start(app.handle().clone(), &app.state::<island::IslandState>())?;
            tray::create(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
