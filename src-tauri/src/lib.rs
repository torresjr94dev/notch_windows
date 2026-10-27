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
    tauri::Builder::default()
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
