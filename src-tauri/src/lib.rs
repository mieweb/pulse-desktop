mod commands;
mod events;
mod state;
mod capture;
mod hotkey;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::set_output_folder,
            commands::set_mic_enabled,
            commands::authorize_capture,
            commands::get_output_folder,
            commands::init_hotkey,
            commands::start_recording,
            commands::stop_recording,
            commands::open_folder,
            commands::open_file,
            commands::set_capture_region,
            commands::clear_capture_region,
            commands::get_capture_region,
            commands::open_region_selector,
            commands::close_region_selector,
        ])
        .setup(|app| {
            commands::setup_global_shortcut(&app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
