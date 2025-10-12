use tauri::{State, AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri::utils::config::Color;
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
                        
                        // Get mic_enabled from state
                        let mic_enabled = {
                            let mic = state.mic_enabled.lock().unwrap();
                            *mic
                        };
                        
                        // Get capture region from state
                        let capture_region = {
                            let region = state.capture_region.lock().unwrap();
                            *region
                        };
                        
                        // Create capturer
                        let mut capturer = ScreenCapturer::new(output_folder, mic_enabled);
                        
                        // Start recording (blocking call)
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        match runtime.block_on(capturer.start_recording(capture_region)) {
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
    Ok("Recording stopped".to_string())
}

/// Open a folder in the system file explorer
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

/// Open a file with the system's default application
#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    Ok(())
}

/// Set capture region for recording
#[tauri::command]
pub async fn set_capture_region(
    state: State<'_, AppState>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut region = state.capture_region.lock().unwrap();
    *region = Some((x, y, width, height));
    println!("📏 Capture region set: {}x{} at ({}, {})", width, height, x, y);
    Ok(())
}

/// Clear capture region (return to full screen)
#[tauri::command]
pub async fn clear_capture_region(state: State<'_, AppState>) -> Result<(), String> {
    let mut region = state.capture_region.lock().unwrap();
    *region = None;
    println!("🖥️ Capture region cleared - using full screen");
    Ok(())
}

/// Get current capture region
#[tauri::command]
pub async fn get_capture_region(
    state: State<'_, AppState>,
) -> Result<Option<(u32, u32, u32, u32)>, String> {
    let region = state.capture_region.lock().unwrap();
    Ok(*region)
}

/// Open region selector window covering entire screen
#[tauri::command]
pub async fn open_region_selector(
    app: AppHandle,
    aspect_ratio: String,
    scale_to_preset: bool,
) -> Result<(), String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    
    println!("📏 Opening region selector overlay");
    
    // Create full-screen overlay window using the main app with a special query parameter
    let url = format!("{}?mode=region-selector&aspectRatio={}&scaleToPreset={}", 
                     "http://localhost:1420", aspect_ratio, scale_to_preset);
    
    let _window = WebviewWindowBuilder::new(
        &app,
        "region_selector",
        WebviewUrl::External(url.parse().unwrap())
    )
    .title("Select Capture Region")
    .resizable(false)
    .maximized(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .background_color((0, 0, 0, 0).into())
    .focused(true)
    .build()
    .map_err(|e| format!("Failed to create region selector window: {}", e))?;
    
    Ok(())
}

/// Close the region selector window
#[tauri::command]
pub async fn close_region_selector(app: AppHandle) -> Result<(), String> {
    println!("🔒 Closing region selector window");
    
    if let Some(window) = app.get_webview_window("region_selector") {
        window.close().map_err(|e| format!("Failed to close region selector window: {}", e))?;
        println!("✅ Region selector window closed");
    } else {
        println!("⚠️ Region selector window not found");
    }
    
    Ok(())
}
