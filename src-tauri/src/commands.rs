use tauri::{State, AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::state::AppState;
use crate::events;
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use std::io::Read;
use log::{debug, info, warn, error};

#[cfg(target_os = "macos")]
use crate::capture::macos::ScreenCapturer;

#[cfg(target_os = "windows")]
use crate::capture::windows::ScreenCapturer;

// Performance thresholds (in milliseconds)
/// Expected maximum time from hotkey press to recording start (includes all overhead)
/// This includes AVAssetWriter initialization, ScreenCaptureKit activation, and thread overhead
pub const HOTKEY_TO_RECORDING_THRESHOLD_MS: u128 = 250;

// Global state to track if we're currently recording
static IS_RECORDING: AtomicBool = AtomicBool::new(false);

// Global state to track if capturer is initializing (prevents recording during re-init)
static IS_INITIALIZING: AtomicBool = AtomicBool::new(false);

// Global state to track if recording has actually started (vs just initiated)
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

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
        debug!("Hotkey event: state={:?}", event.state);
        
        match event.state {
            ShortcutState::Pressed => {
                // Key pressed - start recording
                if !IS_RECORDING.swap(true, Ordering::SeqCst) {
                    // Check if capturer is still initializing
                    if IS_INITIALIZING.load(Ordering::SeqCst) {
                        warn!("⏳ Capturer still initializing from previous recording - please wait");
                        IS_RECORDING.store(false, Ordering::SeqCst);
                        return;
                    }
                    
                    let press_time = std::time::Instant::now();
                    info!("🎬 Starting recording at {:?}...", press_time);
                    
                    // Check if we have a current project before starting
                    let state = app.state::<AppState>();
                    let current_project = {
                        let project = state.current_project.lock().unwrap();
                        project.clone()
                    };
                    
                    // If no project is selected, emit event and stop recording
                    if current_project.is_none() {
                        warn!("⚠️  No project selected - requesting project name");
                        let _ = events::emit_project_required(app);
                        IS_RECORDING.store(false, Ordering::SeqCst);
                        return;
                    }
                    
                    // Pause filesystem watcher during recording
                    {
                        let control = state.watcher_control.lock().unwrap();
                        if let Some(watcher) = control.as_ref() {
                            watcher.pause();
                        }
                    }
                    
                    let _ = events::emit_status(app, "recording");
                    
                    // Check if we already have a pre-initialized capturer
                    let capturer_ready = {
                        let cap = state.capturer.lock().unwrap();
                        cap.is_some()
                    };
                    
                    if capturer_ready {
                        // Fast path: capturer already initialized
                        info!("⚡ Using pre-initialized capturer (fast path)");
                        let app_clone = app.clone();
                        std::thread::spawn(move || {
                            let state = app_clone.state::<AppState>();
                            
                            // Get capture region from state
                            let capture_region = {
                                let region = state.capture_region.lock().unwrap();
                                *region
                            };
                            
                            // Take capturer from state
                            let capturer_option = {
                                let mut cap = state.capturer.lock().unwrap();
                                cap.take()
                            };
                            
                            if let Some(mut capturer) = capturer_option {
                                let runtime = tokio::runtime::Runtime::new().unwrap();
                                match runtime.block_on(capturer.start_recording(capture_region)) {
                                    Ok(_) => {
                                        let elapsed = press_time.elapsed();
                                        info!("✅ Screen capture started in {:?}", elapsed);
                                        
                                        // Mark recording as actually active now
                                        RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                        
                                        if elapsed.as_millis() > HOTKEY_TO_RECORDING_THRESHOLD_MS {
                                            error!("⚠️  SLOW START DETECTED: {:?} from key press to recording started", elapsed);
                                            error!("💔 We sincerely apologize - you may have lost the first {:?} of your recording.", elapsed);
                                            error!("🔧 This should not happen with pre-initialization. Please report this issue.");
                                        }
                                        
                                        // Store capturer back in state
                                        let mut cap = state.capturer.lock().unwrap();
                                        *cap = Some(capturer);
                                    }
                                    Err(e) => {
                                        error!("❌ Failed to start recording: {}", e);
                                        let _ = events::emit_error(&app_clone, "CAPTURE_ERROR", &e);
                                        IS_RECORDING.store(false, Ordering::SeqCst);
                                        RECORDING_ACTIVE.store(false, Ordering::SeqCst); // Clear active flag on error
                                        let _ = events::emit_status(&app_clone, "idle");
                                        
                                        // Resume filesystem watcher on error
                                        {
                                            let control = state.watcher_control.lock().unwrap();
                                            if let Some(watcher) = control.as_ref() {
                                                watcher.resume();
                                            }
                                        }
                                    }
                                }
                            } else {
                                error!("❌ Capturer was expected but not found!");
                                let _ = events::emit_error(&app_clone, "CAPTURE_ERROR", "Capturer initialization failed");
                                IS_RECORDING.store(false, Ordering::SeqCst);
                                RECORDING_ACTIVE.store(false, Ordering::SeqCst); // Clear active flag on error
                                let _ = events::emit_status(&app_clone, "idle");
                                
                                // Resume filesystem watcher
                                {
                                    let control = state.watcher_control.lock().unwrap();
                                    if let Some(watcher) = control.as_ref() {
                                        watcher.resume();
                                    }
                                }
                            }
                        });
                    } else {
                        // Slow path: need to create capturer (this should rarely happen)
                        warn!("🐌 SLOW PATH: Creating capturer on demand (this should not happen!)");
                        error!("⚠️  CAPTURER NOT PRE-INITIALIZED!");
                        error!("💔 We sincerely apologize - you will likely lose the first few seconds of your recording.");
                        error!("🔧 Please ensure a project is selected before recording to enable fast startup.");
                        
                        let app_clone = app.clone();
                        std::thread::spawn(move || {
                            let state = app_clone.state::<AppState>();
                            
                            // Get output folder and current project
                            let (base_output_folder, current_project) = {
                                let folder = state.output_folder.lock().unwrap();
                                let project = state.current_project.lock().unwrap();
                                (folder.clone(), project.clone())
                            };
                            
                            // At this point we know current_project is Some(), so unwrap is safe
                            let project_name = current_project.unwrap();
                            let output_folder = base_output_folder.join(&project_name);
                            
                            // Create output folder if it doesn't exist
                            if let Err(e) = std::fs::create_dir_all(&output_folder) {
                                error!("Failed to create output folder: {}", e);
                                let _ = events::emit_error(&app_clone, "FOLDER_ERROR", &format!("Failed to create output folder: {}", e));
                                IS_RECORDING.store(false, Ordering::SeqCst);
                                let _ = events::emit_status(&app_clone, "idle");
                                
                                // Resume filesystem watcher on error
                                {
                                    let control = state.watcher_control.lock().unwrap();
                                    if let Some(watcher) = control.as_ref() {
                                        watcher.resume();
                                    }
                                }
                                return;
                            }
                            
                            // Get mic_enabled from state
                            let mic_enabled = {
                                let mic = state.mic_enabled.lock().unwrap();
                                *mic
                            };
                            
                            // Get selected audio device from state
                            let audio_device_id = {
                                let device = state.selected_audio_device.lock().unwrap();
                                device.clone()
                            };
                            
                            // Get capture region from state
                            let capture_region = {
                                let region = state.capture_region.lock().unwrap();
                                *region
                            };
                            
                            // Create capturer
                            let mut capturer = ScreenCapturer::new(output_folder, mic_enabled, audio_device_id);
                            
                            // Start recording (blocking call)
                            let runtime = tokio::runtime::Runtime::new().unwrap();
                            match runtime.block_on(capturer.start_recording(capture_region)) {
                                Ok(_) => {
                                    let elapsed = press_time.elapsed();
                                    info!("✅ Screen capture started in {:?}", elapsed);
                                    error!("⏱️  Slow initialization took {:?} - user lost beginning of recording", elapsed);
                                    
                                    // Mark recording as active
                                    RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                    
                                    // Store capturer in state
                                    let mut cap = state.capturer.lock().unwrap();
                                    *cap = Some(capturer);
                                }
                                Err(e) => {
                                    error!("❌ Failed to start recording: {}", e);
                                    let _ = events::emit_error(&app_clone, "CAPTURE_ERROR", &e);
                                    IS_RECORDING.store(false, Ordering::SeqCst);
                                    RECORDING_ACTIVE.store(false, Ordering::SeqCst); // Clear active flag on error
                                    let _ = events::emit_status(&app_clone, "idle");
                                    
                                    // Resume filesystem watcher on error
                                    {
                                        let control = state.watcher_control.lock().unwrap();
                                        if let Some(watcher) = control.as_ref() {
                                            watcher.resume();
                                        }
                                    }
                                }
                            }
                        });
                    }
                } else {
                    warn!("⚠️  Already recording, ignoring press");
                }
            }
            ShortcutState::Released => {
                // Key released - stop recording
                if IS_RECORDING.swap(false, Ordering::SeqCst) {
                    // Check if recording has actually started
                    if !RECORDING_ACTIVE.load(Ordering::SeqCst) {
                        warn!("⏳ Recording initiated but not yet started - waiting for it to begin...");
                        // Wait a bit for recording to actually start (max 200ms)
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        
                        // Check again
                        if !RECORDING_ACTIVE.load(Ordering::SeqCst) {
                            warn!("⚠️  Recording never started, canceling...");
                            let _ = events::emit_status(app, "idle");
                            return;
                        }
                    }
                    
                    info!("⏹️  Stopping recording...");
                    
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
                                Ok((path, duration_seconds)) => {
                                    // Clear recording active flag
                                    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                                    
                                    // Convert duration to milliseconds
                                    let duration_ms = (duration_seconds * 1000.0) as u64;
                                    
                                    info!("✅ Recording saved to: {:?}, duration: {:.2}s", path, duration_seconds);
                                    
                                    // Increment clip count
                                    {
                                        let mut count = state.clip_count.lock().unwrap();
                                        *count += 1;
                                    }
                                    
                                    // Add timeline entry if we have a current project
                                    let current_project = {
                                        let project = state.current_project.lock().unwrap();
                                        project.clone()
                                    };
                                    
                                    if let Some(_project_name) = current_project {
                                        let filename = path.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("recording.mp4")
                                            .to_string();
                                        
                                        // Use actual duration from recording
                                        let aspect_ratio = "none".to_string(); // Will be updated with actual aspect ratio
                                        let width = 1920; // Will be updated with actual width
                                        let height = 1080; // Will be updated with actual height
                                        
                                        // Add timeline entry using the runtime we have  
                                        let app_handle_clone = app_clone.clone();
                                        let _ = runtime.block_on(async {
                                            if let Err(e) = add_timeline_entry(
                                                filename,
                                                duration_ms, // Now using actual duration
                                                aspect_ratio,
                                                width,
                                                height,
                                                app_handle_clone.state::<AppState>()
                                            ).await {
                                                error!("Failed to add timeline entry: {}", e);
                                            }
                                        });
                                    }
                                    
                                    // Emit clip saved event with actual duration
                                    let _ = events::emit_clip_saved(&app_clone, events::ClipSavedEvent {
                                        path: path.to_string_lossy().to_string(),
                                        duration_ms, // Now using actual duration
                                    });
                                    
                                    // Re-initialize capturer for next recording (in background)
                                    {
                                        let current_project = state.current_project.lock().unwrap().clone();
                                        if let Some(_project_name) = current_project {
                                            info!("🔄 Re-initializing capturer for next recording...");
                                            
                                            // Set initializing flag to prevent recording during re-init
                                            IS_INITIALIZING.store(true, Ordering::SeqCst);
                                            
                                            let output_folder = state.output_folder.lock().unwrap().clone();
                                            let mic_enabled = state.mic_enabled.lock().unwrap().clone();
                                            let audio_device_id = state.selected_audio_device.lock().unwrap().clone();
                                            let capture_region = state.capture_region.lock().unwrap().clone();
                                            let project_folder = output_folder.join(_project_name);
                                            
                                            // Create new capturer
                                            let mut new_capturer = ScreenCapturer::new(project_folder, mic_enabled, audio_device_id);
                                            
                                            // Pre-initialize in background using Tauri's async runtime
                                            let app_for_spawn = app_clone.clone();
                                            tauri::async_runtime::spawn(async move {
                                                match new_capturer.pre_initialize(capture_region).await {
                                                    Ok(_) => {
                                                        info!("✅ Capturer re-initialized for next recording");
                                                        // Store back in state
                                                        let state = app_for_spawn.state::<AppState>();
                                                        let mut cap = state.capturer.lock().unwrap();
                                                        *cap = Some(new_capturer);
                                                        
                                                        // Clear initializing flag - ready to record again
                                                        IS_INITIALIZING.store(false, Ordering::SeqCst);
                                                    }
                                                    Err(e) => {
                                                        warn!("⚠️  Failed to re-initialize capturer: {}", e);
                                                        // Clear initializing flag even on error
                                                        IS_INITIALIZING.store(false, Ordering::SeqCst);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    
                                    // Resume filesystem watcher AFTER clip is fully processed and event emitted
                                    {
                                        let control = state.watcher_control.lock().unwrap();
                                        if let Some(watcher) = control.as_ref() {
                                            watcher.resume();
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("❌ Failed to stop recording: {}", e);
                                    let _ = events::emit_error(&app_clone, "SAVE_ERROR", &e);
                                    
                                    // Clear recording active flag even on error
                                    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
                                    
                                    // Resume filesystem watcher even on error
                                    {
                                        let control = state.watcher_control.lock().unwrap();
                                        if let Some(watcher) = control.as_ref() {
                                            watcher.resume();
                                        }
                                    }
                                }
                            }
                        } else {
                            warn!("⚠️  No active capturer to stop");
                            RECORDING_ACTIVE.store(false, Ordering::SeqCst); // Clear flag even if no capturer
                        }
                    });
                } else {
                    warn!("⚠️  Not recording, ignoring release");
                }
            }
        }
    })?;
    
    info!("✅ Global shortcut registered: {}", shortcut);
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
pub async fn set_mic_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut mic = state.mic_enabled.lock()
            .map_err(|e| format!("Failed to lock mic_enabled: {}", e))?;
        *mic = enabled;
    }
    
    // Re-initialize capturer with new mic setting if we have a project selected
    reinitialize_capturer_if_needed(state).await?;
    
    Ok(())
}

/// Get available audio input devices
#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<screen_capture::AudioDevice>, String> {
    screen_capture::get_audio_devices()
}

/// Set selected audio device
#[tauri::command]
pub async fn set_audio_device(device_id: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut device = state.selected_audio_device.lock()
            .map_err(|e| format!("Failed to lock selected_audio_device: {}", e))?;
        *device = Some(device_id.clone());
    }
    
    info!("🎤 Audio device changed to: {}", device_id);
    
    // Re-initialize capturer with new device if we have a project selected
    reinitialize_capturer_if_needed(state).await?;
    
    Ok(())
}

/// Helper function to re-initialize capturer when settings change
async fn reinitialize_capturer_if_needed(state: State<'_, AppState>) -> Result<(), String> {
    let current_project = {
        let project = state.current_project.lock()
            .map_err(|e| format!("Failed to lock current_project: {}", e))?;
        project.clone()
    };
    
    if let Some(project_name) = current_project {
        info!("🔄 Re-initializing capturer due to settings change...");
        
        let output_folder = {
            let folder = state.output_folder.lock()
                .map_err(|e| format!("Failed to lock output_folder: {}", e))?;
            folder.clone()
        };
        
        let output_path = output_folder.join(&project_name);
        
        let mic_enabled = {
            let mic = state.mic_enabled.lock()
                .map_err(|e| format!("Failed to lock mic_enabled: {}", e))?;
            *mic
        };
        
        let audio_device_id = {
            let device = state.selected_audio_device.lock()
                .map_err(|e| format!("Failed to lock selected_audio_device: {}", e))?;
            device.clone()
        };
        
        let capture_region = {
            let region = state.capture_region.lock()
                .map_err(|e| format!("Failed to lock capture_region: {}", e))?;
            *region
        };
        
        // Create and pre-initialize new capturer
        let mut capturer = ScreenCapturer::new(output_path, mic_enabled, audio_device_id);
        capturer.pre_initialize(capture_region).await
            .map_err(|e| format!("Failed to re-initialize recorder: {}", e))?;
        
        info!("✅ Capturer re-initialized");
        
        // Store in state
        {
            let mut cap = state.capturer.lock()
                .map_err(|e| format!("Failed to lock capturer: {}", e))?;
            *cap = Some(capturer);
        }
    }
    
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
    debug!("Initializing global hotkey...");
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
    debug!("Manual start recording triggered");
    // TODO: Implement actual recording start
    Ok(())
}

/// Stop recording manually (for testing without hotkey)
#[tauri::command]
pub async fn stop_recording(_state: State<'_, AppState>) -> Result<String, String> {
    debug!("Manual stop recording triggered");
    // TODO: Implement actual recording stop
    Ok("Recording stopped".to_string())
}

/// Performance settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    /// Expected maximum time from hotkey press to recording start (includes all overhead)
    pub hotkey_to_recording_threshold_ms: u128,
}

/// Get performance thresholds
#[tauri::command]
pub async fn get_performance_settings() -> Result<PerformanceSettings, String> {
    Ok(PerformanceSettings {
        hotkey_to_recording_threshold_ms: HOTKEY_TO_RECORDING_THRESHOLD_MS,
    })
}

/// Open a folder in the system file explorer
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    // Expand ~ to home directory
    let expanded_path = if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home_path = std::path::PathBuf::from(home);
            home_path.join(&path[2..]).to_string_lossy().to_string()
        } else {
            path.clone()
        }
    } else {
        path.clone()
    };
    
    // Verify folder exists
    if !std::path::Path::new(&expanded_path).exists() {
        return Err(format!("The folder {} does not exist.", expanded_path));
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&expanded_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&expanded_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&expanded_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

/// Open a file with the system's default application
#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    // Expand ~ to home directory
    let expanded_path = if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let home_path = std::path::PathBuf::from(home);
            home_path.join(&path[2..]).to_string_lossy().to_string()
        } else {
            path.clone()
        }
    } else {
        path.clone()
    };
    
    // Verify file exists
    if !std::path::Path::new(&expanded_path).exists() {
        return Err(format!("The file {} does not exist.", expanded_path));
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&expanded_path)
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", "", &expanded_path])
            .spawn()
            .map_err(|e| format!("Failed to open file: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&expanded_path)
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
    {
        let mut region = state.capture_region.lock().unwrap();
        *region = Some((x, y, width, height));
        info!("📏 Capture region set: {}x{} at ({}, {})", width, height, x, y);
    }
    
    // Re-initialize capturer with new region
    reinitialize_capturer_if_needed(state).await?;
    
    Ok(())
}

/// Clear capture region (return to full screen)
#[tauri::command]
pub async fn clear_capture_region(state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut region = state.capture_region.lock().unwrap();
        *region = None;
        info!("🖥️ Capture region cleared - using full screen");
    }
    
    // Re-initialize capturer for full screen
    reinitialize_capturer_if_needed(state).await?;
    
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
    
    debug!("📏 Opening region selector overlay");
    
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
    debug!("🔒 Closing region selector window");
    
    if let Some(window) = app.get_webview_window("region_selector") {
        window.close().map_err(|e| format!("Failed to close region selector window: {}", e))?;
        debug!("✅ Region selector window closed");
    } else {
        debug!("⚠️ Region selector window not found");
    }
    
    Ok(())
}

// Project management structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub created_at: String,
    pub video_count: u32,
    pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: String,
    pub filename: String,
    #[serde(rename = "recordedAt", alias = "recorded_at")]
    pub recorded_at: String,
    #[serde(rename = "durationMs", alias = "duration_ms")]
    pub duration_ms: u64,
    pub aspect_ratio: String,
    pub resolution: Resolution,
    pub mic_enabled: bool,
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>, // SHA256 hash for file integrity and rename detection
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTimeline {
    #[serde(rename = "projectName", alias = "project_name")]
    pub project_name: String,
    #[serde(rename = "createdAt", alias = "created_at")]
    pub created_at: String,
    #[serde(rename = "lastModified", alias = "last_modified")]
    pub last_modified: String,
    pub entries: Vec<TimelineEntry>,
    pub metadata: TimelineMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMetadata {
    #[serde(rename = "totalVideos", alias = "total_videos")]
    pub total_videos: u32,
    #[serde(rename = "totalDuration", alias = "total_duration")]
    pub total_duration: u64,
    #[serde(rename = "defaultAspectRatio", alias = "default_aspect_ratio")]
    pub default_aspect_ratio: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Get all available projects in the output folder
#[tauri::command]
pub async fn get_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let mut projects = Vec::new();

    if !output_folder.exists() {
        return Ok(projects);
    }

    let entries = fs::read_dir(&output_folder).map_err(|e| format!("Failed to read output folder: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let project_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            // Check for timeline.json file
            let timeline_path = path.join("timeline.json");
            
            if timeline_path.exists() {
                // Read timeline to get project info
                match fs::read_to_string(&timeline_path) {
                    Ok(content) => {
                        match serde_json::from_str::<ProjectTimeline>(&content) {
                            Ok(timeline) => {
                                projects.push(Project {
                                    name: project_name,
                                    created_at: timeline.created_at,
                                    video_count: timeline.metadata.total_videos,
                                    last_modified: timeline.last_modified,
                                });
                            }
                            Err(_) => {
                                // Invalid timeline, create default project entry
                                let now = chrono::Utc::now().to_rfc3339();
                                projects.push(Project {
                                    name: project_name,
                                    created_at: now.clone(),
                                    video_count: 0,
                                    last_modified: now,
                                });
                            }
                        }
                    }
                    Err(_) => {
                        // Can't read timeline, create default project entry
                        let now = chrono::Utc::now().to_rfc3339();
                        projects.push(Project {
                            name: project_name,
                            created_at: now.clone(),
                            video_count: 0,
                            last_modified: now,
                        });
                    }
                }
            } else {
                // No timeline file, create default project entry
                let now = chrono::Utc::now().to_rfc3339();
                projects.push(Project {
                    name: project_name,
                    created_at: now.clone(),
                    video_count: 0,
                    last_modified: now,
                });
            }
        }
    }

    // Sort projects by last modified date (newest first)
    projects.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(projects)
}

/// Create a new project
#[tauri::command]
pub async fn create_project(project_name: String, state: State<'_, AppState>) -> Result<(), String> {
    if project_name.trim().is_empty() {
        return Err("Project name cannot be empty".to_string());
    }

    let project_name = project_name.trim().to_string();

    // Validate project name (no special characters that would cause filesystem issues)
    if project_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Project name contains invalid characters".to_string());
    }

    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let project_folder = output_folder.join(&project_name);

    if project_folder.exists() {
        return Err("Project already exists".to_string());
    }

    // Create project folder
    fs::create_dir_all(&project_folder).map_err(|e| format!("Failed to create project folder: {}", e))?;

    // Create initial timeline.json
    let now = chrono::Utc::now().to_rfc3339();
    let timeline = ProjectTimeline {
        project_name: project_name.clone(),
        created_at: now.clone(),
        last_modified: now,
        entries: Vec::new(),
        metadata: TimelineMetadata {
            total_videos: 0,
            total_duration: 0,
            default_aspect_ratio: None,
            tags: None,
        },
    };

    let timeline_path = project_folder.join("timeline.json");
    let timeline_json = serde_json::to_string_pretty(&timeline)
        .map_err(|e| format!("Failed to serialize timeline: {}", e))?;

    fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Failed to write timeline.json: {}", e))?;

    // Set as current project
    {
        let mut current = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
        *current = Some(project_name);
    }

    Ok(())
}

/// Set the current project
#[tauri::command]
pub async fn set_current_project(project_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let project_folder = output_folder.join(&project_name);

    if !project_folder.exists() {
        return Err("Project does not exist".to_string());
    }

    {
        let mut current = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
        *current = Some(project_name.clone());
    }
    
    // Pre-initialize capturer for instant recording startup
    info!("⚡ Pre-initializing capturer for project: {}", project_name);
    
    // Set initializing flag
    IS_INITIALIZING.store(true, Ordering::SeqCst);
    
    let output_path = output_folder.join(&project_name);
    
    let mic_enabled = {
        let mic = state.mic_enabled.lock().map_err(|e| format!("Failed to lock mic_enabled: {}", e))?;
        *mic
    };
    
    let audio_device_id = {
        let device = state.selected_audio_device.lock().map_err(|e| format!("Failed to lock selected_audio_device: {}", e))?;
        device.clone()
    };
    
    let capture_region = {
        let region = state.capture_region.lock().map_err(|e| format!("Failed to lock capture_region: {}", e))?;
        *region
    };
    
    // Create capturer
    let mut capturer = ScreenCapturer::new(output_path, mic_enabled, audio_device_id);
    
    // Pre-initialize (this takes 2-3 seconds)
    capturer.pre_initialize(capture_region).await
        .map_err(|e| {
            warn!("⚠️  Failed to pre-initialize capturer: {}", e);
            IS_INITIALIZING.store(false, Ordering::SeqCst); // Clear flag on error
            format!("Failed to pre-initialize recorder: {}", e)
        })?;
    
    info!("✅ Capturer pre-initialized and ready for instant recording");
    
    // Store in state
    {
        let mut cap = state.capturer.lock().map_err(|e| format!("Failed to lock capturer: {}", e))?;
        *cap = Some(capturer);
    }
    
    // Clear initializing flag - ready to record
    IS_INITIALIZING.store(false, Ordering::SeqCst);

    Ok(())
}

/// Get the current project
#[tauri::command]
pub async fn get_current_project(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let current = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
    Ok(current.clone())
}

/// Get timeline for a specific project
#[tauri::command]
pub async fn get_project_timeline(project_name: String, state: State<'_, AppState>) -> Result<ProjectTimeline, String> {
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let timeline_path = output_folder.join(&project_name).join("timeline.json");

    if !timeline_path.exists() {
        return Err("Timeline not found".to_string());
    }

    let content = fs::read_to_string(&timeline_path)
        .map_err(|e| format!("Failed to read timeline: {}", e))?;

    let timeline: ProjectTimeline = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse timeline: {}", e))?;

    Ok(timeline)
}

/// Calculate SHA256 checksum of a file
fn calculate_file_checksum(file_path: &PathBuf) -> Result<String, String> {
    let mut file = fs::File::open(file_path)
        .map_err(|e| format!("Failed to open file for checksum: {}", e))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer for efficient reading
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .map_err(|e| format!("Failed to read file for checksum: {}", e))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Save timeline for a specific project
#[tauri::command]
pub async fn save_project_timeline(project_name: String, timeline: ProjectTimeline, state: State<'_, AppState>) -> Result<(), String> {
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let timeline_path = output_folder.join(&project_name).join("timeline.json");

    // Update lastModified timestamp
    let mut updated_timeline = timeline;
    updated_timeline.last_modified = chrono::Utc::now().to_rfc3339();

    let json = serde_json::to_string_pretty(&updated_timeline)
        .map_err(|e| format!("Failed to serialize timeline: {}", e))?;

    fs::write(&timeline_path, json)
        .map_err(|e| format!("Failed to write timeline: {}", e))?;

    Ok(())
}

/// Reconcile timeline with actual files in project folder
/// This function:
/// 1. Detects deleted files and removes them from timeline
/// 2. Detects new files and adds them to timeline
/// 3. Uses checksums to detect renamed files
/// 4. Updates/calculates checksums for all files
#[tauri::command]
pub async fn reconcile_project_timeline(project_name: String, state: State<'_, AppState>) -> Result<u32, String> {
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let project_folder = output_folder.join(&project_name);
    let timeline_path = project_folder.join("timeline.json");

    if !project_folder.exists() {
        return Err("Project folder does not exist".to_string());
    }

    // Read existing timeline or create new one
    let mut timeline = if timeline_path.exists() {
        let content = fs::read_to_string(&timeline_path)
            .map_err(|e| format!("Failed to read timeline: {}", e))?;
        serde_json::from_str::<ProjectTimeline>(&content)
            .map_err(|e| format!("Failed to parse timeline: {}", e))?
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        ProjectTimeline {
            project_name: project_name.clone(),
            created_at: now.clone(),
            last_modified: now,
            entries: Vec::new(),
            metadata: TimelineMetadata {
                total_videos: 0,
                total_duration: 0,
                default_aspect_ratio: None,
                tags: None,
            },
        }
    };

    // Get all video files in the project folder with their checksums
    let entries = fs::read_dir(&project_folder).map_err(|e| format!("Failed to read project folder: {}", e))?;
    
    let mut actual_files: HashMap<String, String> = HashMap::new(); // filename -> checksum
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(extension) = path.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                if matches!(ext_str.as_str(), "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v") {
                    if let Some(filename) = path.file_name() {
                        let filename_str = filename.to_string_lossy().to_string();
                        // Calculate checksum for the file
                        if let Ok(checksum) = calculate_file_checksum(&path) {
                            actual_files.insert(filename_str, checksum);
                        } else {
                            warn!("⚠️  Failed to calculate checksum for {}", filename_str);
                        }
                    }
                }
            }
        }
    }

    let mut changes_count = 0u32;

    // Step 1: Remove deleted files from timeline
    let original_count = timeline.entries.len();
    timeline.entries.retain(|entry| {
        let exists = actual_files.contains_key(&entry.filename);
        if !exists {
            info!("🗑️  Removed deleted file from timeline: {}", entry.filename);
        }
        exists
    });
    let deleted_count = original_count - timeline.entries.len();
    changes_count += deleted_count as u32;

    // Step 2: Build checksum maps for rename detection
    let mut timeline_checksums: HashMap<String, usize> = HashMap::new(); // checksum -> index in entries
    for (idx, entry) in timeline.entries.iter().enumerate() {
        if let Some(checksum) = &entry.checksum {
            timeline_checksums.insert(checksum.clone(), idx);
        }
    }

    let mut actual_checksums: HashMap<String, String> = HashMap::new(); // checksum -> filename
    for (filename, checksum) in &actual_files {
        actual_checksums.insert(checksum.clone(), filename.clone());
    }

    // Step 3: Detect renames by matching checksums
    for entry in timeline.entries.iter_mut() {
        if let Some(old_checksum) = &entry.checksum {
            // If the current filename doesn't exist but checksum matches another file
            if !actual_files.contains_key(&entry.filename) {
                if let Some(new_filename) = actual_checksums.get(old_checksum) {
                    info!("📝 Detected rename: {} -> {}", entry.filename, new_filename);
                    entry.filename = new_filename.clone();
                    entry.notes = Some(format!("File renamed during reconciliation"));
                    changes_count += 1;
                }
            }
        }
    }

    // Step 4: Update checksums for existing entries
    for entry in timeline.entries.iter_mut() {
        if let Some(checksum) = actual_files.get(&entry.filename) {
            if entry.checksum.is_none() || entry.checksum.as_ref() != Some(checksum) {
                entry.checksum = Some(checksum.clone());
            }
        }
    }

    // Step 5: Add new files that aren't in timeline
    let existing_filenames: std::collections::HashSet<String> = timeline.entries
        .iter()
        .map(|entry| entry.filename.clone())
        .collect();

    for (filename, checksum) in &actual_files {
        if !existing_filenames.contains(filename) {
            let file_path = project_folder.join(filename);
            
            // Try to get file metadata
            let metadata = fs::metadata(&file_path);
            let (created_time, file_size) = if let Ok(meta) = metadata {
                let created = meta.created().unwrap_or(std::time::SystemTime::now());
                let size = meta.len();
                (created, size)
            } else {
                (std::time::SystemTime::now(), 0)
            };

            // Convert system time to RFC3339 string
            let created_rfc3339 = match created_time.duration_since(std::time::UNIX_EPOCH) {
                Ok(duration) => {
                    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(duration.as_secs() as i64, 0);
                    datetime.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
                }
                Err(_) => chrono::Utc::now().to_rfc3339(),
            };

            // Try to detect video properties from filename patterns
            let (aspect_ratio, width, height) = detect_video_properties_from_filename(filename);

            // Create timeline entry for new file
            let entry_id = uuid::Uuid::new_v4().to_string();
            let entry = TimelineEntry {
                id: entry_id,
                filename: filename.clone(),
                recorded_at: created_rfc3339,
                duration_ms: estimate_duration_from_file_size(file_size),
                aspect_ratio,
                resolution: Resolution { width, height },
                mic_enabled: true,
                notes: Some("Added during timeline reconciliation".to_string()),
                checksum: Some(checksum.clone()),
            };

            info!("➕ Added new file to timeline: {}", filename);
            timeline.entries.push(entry);
            changes_count += 1;
        }
    }

    // Sort entries by recorded_at timestamp
    timeline.entries.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at));

    // Update timeline metadata
    timeline.last_modified = chrono::Utc::now().to_rfc3339();
    timeline.metadata.total_videos = timeline.entries.len() as u32;
    timeline.metadata.total_duration = timeline.entries.iter().map(|e| e.duration_ms).sum();

    // Save updated timeline
    let timeline_json = serde_json::to_string_pretty(&timeline)
        .map_err(|e| format!("Failed to serialize timeline: {}", e))?;

    fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Failed to write timeline.json: {}", e))?;

    info!("✅ Reconciliation complete: {} changes detected", changes_count);
    Ok(changes_count)
}

// Helper function to detect video properties from filename
fn detect_video_properties_from_filename(filename: &str) -> (String, u32, u32) {
    let lower = filename.to_lowercase();
    
    // Look for common resolution patterns in filename
    if lower.contains("1920x1080") || lower.contains("1080p") || lower.contains("fhd") {
        ("16:9".to_string(), 1920, 1080)
    } else if lower.contains("2560x1440") || lower.contains("1440p") || lower.contains("qhd") {
        ("16:9".to_string(), 2560, 1440)
    } else if lower.contains("3840x2160") || lower.contains("4k") || lower.contains("uhd") {
        ("16:9".to_string(), 3840, 2160)
    } else if lower.contains("1080x1920") || lower.contains("vertical") || lower.contains("portrait") {
        ("9:16".to_string(), 1080, 1920)
    } else if lower.contains("1440x2560") {
        ("9:16".to_string(), 1440, 2560)
    } else if lower.contains("2160x3840") {
        ("9:16".to_string(), 2160, 3840)
    } else {
        // Default to common recording resolution
        ("16:9".to_string(), 1920, 1080)
    }
}

// Helper function to estimate duration from file size (very rough)
fn estimate_duration_from_file_size(file_size: u64) -> u64 {
    // Very rough estimate: assume ~1MB per second for typical screen recordings
    // This is just a placeholder until we can read actual video metadata
    let estimated_seconds = (file_size / (1024 * 1024)).max(1); // At least 1 second
    estimated_seconds * 1000 // Convert to milliseconds
}

/// Add a recording entry to the current project's timeline
#[tauri::command]
pub async fn add_timeline_entry(
    filename: String,
    duration_ms: u64,
    aspect_ratio: String,
    width: u32,
    height: u32,
    state: State<'_, AppState>
) -> Result<(), String> {
    let (output_folder, current_project) = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        let project = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
        (folder.clone(), project.clone())
    };

    let project_name = current_project.ok_or("No current project set")?;
    let timeline_path = output_folder.join(&project_name).join("timeline.json");

    // Read existing timeline or create new one
    let mut timeline = if timeline_path.exists() {
        let content = fs::read_to_string(&timeline_path)
            .map_err(|e| format!("Failed to read timeline: {}", e))?;
        serde_json::from_str::<ProjectTimeline>(&content)
            .map_err(|e| format!("Failed to parse timeline: {}", e))?
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        ProjectTimeline {
            project_name: project_name.clone(),
            created_at: now.clone(),
            last_modified: now,
            entries: Vec::new(),
            metadata: TimelineMetadata {
                total_videos: 0,
                total_duration: 0,
                default_aspect_ratio: None,
                tags: None,
            },
        }
    };

    // Get mic enabled state
    let mic_enabled = {
        let mic = state.mic_enabled.lock().map_err(|e| format!("Failed to lock mic enabled: {}", e))?;
        *mic
    };

    // Calculate checksum for the newly recorded file
    let file_path = output_folder.join(&project_name).join(&filename);
    let checksum = calculate_file_checksum(&file_path).ok(); // Optional, may fail if file is still being written

    // Create new entry
    let entry_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let entry = TimelineEntry {
        id: entry_id,
        filename,
        recorded_at: now.clone(),
        duration_ms,
        aspect_ratio,
        resolution: Resolution { width, height },
        mic_enabled,
        notes: None,
        checksum,
    };

    // Add entry and update metadata
    timeline.entries.push(entry);
    timeline.last_modified = now;
    timeline.metadata.total_videos = timeline.entries.len() as u32;
    timeline.metadata.total_duration = timeline.entries.iter().map(|e| e.duration_ms).sum();


    // Save updated timeline
    let timeline_json = serde_json::to_string_pretty(&timeline)
        .map_err(|e| format!("Failed to serialize timeline: {}", e))?;

    fs::write(&timeline_path, timeline_json)
        .map_err(|e| format!("Failed to write timeline.json: {}", e))?;

    Ok(())
}
