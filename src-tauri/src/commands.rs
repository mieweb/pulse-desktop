use tauri::{State, AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::state::AppState;
use crate::events;

#[cfg(target_os = "macos")]
use crate::capture::macos::ScreenCapturer;

#[cfg(target_os = "windows")]
use crate::capture::windows::ScreenCapturer;

// Global state to track if we're currently recording
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

/// Setup global shortcut during app initialization
pub fn setup_global_shortcut(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Register Cmd+Shift+R (macOS) or Ctrl+Shift+R (Windows)
    #[cfg(target_os = "macos")]
    let shortcut = "CmdOrCtrl+Shift+R";
    
    #[cfg(target_os = "windows")]
    let shortcut = "Ctrl+Shift+R";
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let shortcut = "Ctrl+Shift+R";
    
    let shortcut: Shortcut = shortcut.parse()?;
    let _app_handle = app.clone();
    
    app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
        println!("Hotkey event: state={:?}", event.state);
        
        match event.state {
            ShortcutState::Pressed => {
                // Key pressed - start recording
                if !IS_RECORDING.swap(true, Ordering::SeqCst) {
                    println!("🎬 Starting recording...");
                    let _ = events::emit_status(app, "recording");
                    
                    // Start actual screen capture in background thread
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        let state = app_clone.state::<AppState>();
                        
                        // Get output folder
                        let output_folder = {
                            let folder = state.output_folder.lock().unwrap();
                            folder.clone()
                        };
                        
                        // Create output folder if it doesn't exist
                        if let Err(e) = std::fs::create_dir_all(&output_folder) {
                            eprintln!("Failed to create output folder: {}", e);
                            let _ = events::emit_error(&app_clone, "FOLDER_ERROR", &format!("Failed to create output folder: {}", e));
                            IS_RECORDING.store(false, Ordering::SeqCst);
                            let _ = events::emit_status(&app_clone, "idle");
                            return;
                        }
                        
                        // Create capturer
                        let mut capturer = ScreenCapturer::new(output_folder);
                        
                        // Start recording (blocking call)
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        match runtime.block_on(capturer.start_recording()) {
                            Ok(_) => {
                                println!("✅ Screen capture started");
                                // Store capturer in state
                                let mut cap = state.capturer.lock().unwrap();
                                *cap = Some(capturer);
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to start recording: {}", e);
                                let _ = events::emit_error(&app_clone, "CAPTURE_ERROR", &e);
                                IS_RECORDING.store(false, Ordering::SeqCst);
                                let _ = events::emit_status(&app_clone, "idle");
                            }
                        }
                    });
                } else {
                    println!("⚠️  Already recording, ignoring press");
                }
            }
            ShortcutState::Released => {
                // Key released - stop recording
                if IS_RECORDING.swap(false, Ordering::SeqCst) {
                    println!("⏹️  Stopping recording...");
                    
                    // Immediately transition to idle to allow rapid re-recording
                    let _ = events::emit_status(app, "idle");
                    
                    // Stop actual screen capture in background thread
                    let app_clone = app.clone();
                    std::thread::spawn(move || {
                        let state = app_clone.state::<AppState>();
                        
                        // Get capturer from state (take ownership to release lock immediately)
                        let capturer_option = {
                            let mut cap = state.capturer.lock().unwrap();
                            cap.take()
                        }; // Lock is released here
                        
                        if let Some(mut capturer) = capturer_option {
                            let runtime = tokio::runtime::Runtime::new().unwrap();
                            match runtime.block_on(capturer.stop_recording()) {
                                Ok(path) => {
                                    println!("✅ Recording saved to: {:?}", path);
                                    
                                    // Increment clip count
                                    {
                                        let mut count = state.clip_count.lock().unwrap();
                                        *count += 1;
                                    }
                                    
                                    // Emit clip saved event
                                    let _ = events::emit_clip_saved(&app_clone, events::ClipSavedEvent {
                                        path: path.to_string_lossy().to_string(),
                                        duration_ms: 1000, // TODO: Calculate actual duration
                                    });
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to stop recording: {}", e);
                                    let _ = events::emit_error(&app_clone, "SAVE_ERROR", &e);
                                }
                            }
                        } else {
                            println!("⚠️  No active capturer to stop");
                        }
                    });
                } else {
                    println!("⚠️  Not recording, ignoring release");
                }
            }
        }
    })?;
    
    println!("✅ Global shortcut registered: {}", shortcut);
    Ok(())
}

/// Set the output folder for recordings
#[tauri::command]
pub fn set_output_folder(path: String, state: State<AppState>) -> Result<(), String> {
    let path_buf = PathBuf::from(path);
    
    // Create directory if it doesn't exist
    if !path_buf.exists() {
        std::fs::create_dir_all(&path_buf)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    
    let mut folder = state.output_folder.lock()
        .map_err(|e| format!("Failed to lock output_folder: {}", e))?;
    *folder = path_buf;
    
    Ok(())
}

/// Set microphone enabled state
#[tauri::command]
pub fn set_mic_enabled(enabled: bool, state: State<AppState>) -> Result<(), String> {
    let mut mic = state.mic_enabled.lock()
        .map_err(|e| format!("Failed to lock mic_enabled: {}", e))?;
    *mic = enabled;
    Ok(())
}

/// Authorize screen capture (macOS specific)
#[tauri::command]
pub async fn authorize_capture() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        // TODO: Implement macOS screen capture authorization
        // This would use ScreenCaptureKit to request permissions
        Ok("Authorization requested. Please grant permission in System Preferences.".to_string())
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        Ok("Screen capture authorization not required on this platform.".to_string())
    }
}

/// Get current output folder
#[tauri::command]
pub fn get_output_folder(state: State<AppState>) -> Result<String, String> {
    let folder = state.output_folder.lock()
        .map_err(|e| format!("Failed to lock output_folder: {}", e))?;
    Ok(folder.to_string_lossy().to_string())
}

/// Initialize the global hotkey for recording
#[tauri::command]
pub async fn init_hotkey() -> Result<String, String> {
    println!("Initializing global hotkey...");
    // TODO: This will be implemented when we add full hotkey support
    #[cfg(target_os = "macos")]
    {
        Ok("Hotkey initialized: Cmd+Shift+R".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        Ok("Hotkey initialized: Ctrl+Shift+R".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Hotkeys not supported on this platform".to_string())
    }
}

/// Start recording manually (for testing without hotkey)
#[tauri::command]
pub async fn start_recording(_state: State<'_, AppState>) -> Result<(), String> {
    println!("Manual start recording triggered");
    // TODO: Implement actual recording start
    Ok(())
}

/// Stop recording manually (for testing without hotkey)
#[tauri::command]
pub async fn stop_recording(_state: State<'_, AppState>) -> Result<String, String> {
    println!("Manual stop recording triggered");
    // TODO: Implement actual recording stop
    Ok("recording-1.mp4".to_string())
}
