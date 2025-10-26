mod commands;
mod events;
mod state;
mod capture;
mod hotkey;
mod fs_watcher;

use state::AppState;
use tauri::Manager;

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
            commands::get_projects,
            commands::create_project,
            commands::set_current_project,
            commands::get_current_project,
            commands::get_project_timeline,
            commands::save_project_timeline,
            commands::add_timeline_entry,
            commands::reconcile_project_timeline,
        ])
        .setup(|app| {
            commands::setup_global_shortcut(&app.handle())?;
            
            // Start filesystem watcher for output folder
            let app_handle = app.handle().clone();
            let output_folder = {
                let state = app.state::<AppState>();
                let folder = state.output_folder.lock().unwrap();
                folder.clone()
            };
            
            if let Err(e) = fs_watcher::watch_output_folder(app_handle, output_folder) {
                eprintln!("⚠️  Failed to start filesystem watcher: {}", e);
            }
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
