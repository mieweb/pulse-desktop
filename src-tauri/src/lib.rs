mod commands;
mod events;
mod state;
mod capture;
mod hotkey;
mod fs_watcher;

pub mod logging;

use state::AppState;
use tauri::Manager;
use log::{info, warn};

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
            commands::get_audio_devices,
            commands::set_audio_device,
            commands::authorize_capture,
            commands::get_output_folder,
            commands::init_hotkey,
            commands::start_recording,
            commands::stop_recording,
            commands::get_performance_settings,
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
            commands::get_pre_init_status,
            commands::get_idle_timeout_mins,
            commands::set_idle_timeout_mins,
            commands::update_activity,
            commands::shutdown_idle_capturer,
            commands::toggle_pre_init,
            commands::on_window_focus_gained,
            commands::on_window_focus_lost,
        ])
        .setup(|app| {
            commands::setup_global_shortcut(&app.handle())?;
            
            // Initialize output folder and create default project
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();
            let output_folder = {
                let folder = state.output_folder.lock().unwrap();
                folder.clone()
            };
            
            // Ensure output folder exists
            if !output_folder.exists() {
                info!("📁 Creating output folder: {:?}", output_folder);
                if let Err(e) = std::fs::create_dir_all(&output_folder) {
                    warn!("⚠️  Failed to create output folder: {}", e);
                } else {
                    info!("✅ Output folder created successfully");
                }
            }
            
            // Create default project if no projects exist
            if output_folder.exists() {
                let default_project_path = output_folder.join("Default");
                if !default_project_path.exists() {
                    info!("📁 Creating default project folder");
                    if let Err(e) = std::fs::create_dir_all(&default_project_path) {
                        warn!("⚠️  Failed to create default project folder: {}", e);
                    } else {
                        // Create timeline.json for default project
                        let timeline_path = default_project_path.join("timeline.json");
                        let now = chrono::Utc::now().to_rfc3339();
                        let default_timeline = serde_json::json!({
                            "projectName": "Default",
                            "createdAt": now,
                            "lastModified": now,
                            "entries": [],
                            "metadata": {
                                "totalVideos": 0,
                                "totalDuration": 0,
                                "defaultAspectRatio": null,
                                "tags": null
                            }
                        });
                        
                        if let Err(e) = std::fs::write(&timeline_path, serde_json::to_string_pretty(&default_timeline).unwrap()) {
                            warn!("⚠️  Failed to create default timeline.json: {}", e);
                        } else {
                            info!("✅ Default project created with timeline.json");
                        }
                    }
                }
            }
            
            // Start filesystem watcher for output folder
            match fs_watcher::watch_output_folder(app_handle.clone(), output_folder) {
                Ok(watcher_control) => {
                    info!("✅ Filesystem watcher started, storing control handle");
                    let mut control = state.watcher_control.lock().unwrap();
                    *control = Some(watcher_control);
                }
                Err(e) => {
                    warn!("⚠️  Failed to start filesystem watcher: {}", e);
                }
            }
            
            // Set up window focus event listeners
            commands::setup_window_focus_listeners(app_handle.clone());
            
            // Start idle timeout checker
            commands::start_idle_checker(app_handle);
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
