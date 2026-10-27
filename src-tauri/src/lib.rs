mod island;
mod tray;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(island::IslandState::default())
        .invoke_handler(tauri::generate_handler![island::set_hit_rect])
        .setup(|app| {
            let window = app
                .get_webview_window("island")
                .ok_or("falta la ventana `island` en tauri.conf.json")?;
            island::start(app.handle().clone(), window)?;
            tray::create(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
