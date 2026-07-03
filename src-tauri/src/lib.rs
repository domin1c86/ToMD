mod commands;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|error| {
                format!("failed to resolve application data directory: {error}")
            })?;
            let state = tauri::async_runtime::block_on(state::AppState::open(app_data_dir))
                .map_err(|error| format!("failed to initialize application state: {error}"))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(commands::handler())
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
