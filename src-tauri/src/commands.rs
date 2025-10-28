use tauri::{State, AppHandle, Manager, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use crate::state::{AppState, PreInitStatus};
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
                                        
                                        // Update last activity
                                        if let Ok(mut activity) = state.last_activity.lock() {
                                            *activity = Instant::now();
                                        }
                                        
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
                        // Auto-initialize path: capturer was sleeping, wake it up automatically
                        info!("💤 Capturer was sleeping - auto-initializing for recording...");
                        warn!("⏳ Auto-initialization will add ~2-3 seconds to recording start time");
                        
                        // Emit status change to show we're waking up
                        let _ = events::emit_pre_init_status(app, "Initializing");
                        
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
                            
                            // Pre-initialize capturer before starting recording (blocking call)
                            let runtime = tokio::runtime::Runtime::new().unwrap();
                            match runtime.block_on(capturer.pre_initialize(capture_region)) {
                                Ok(()) => {
                                    // Now start recording
                                    match runtime.block_on(capturer.start_recording(capture_region)) {
                                        Ok(_) => {
                                            let elapsed = press_time.elapsed();
                                            info!("✅ Screen capture started in {:?}", elapsed);
                                            info!("⏰ Auto-initialization took {:?} - recording started successfully", elapsed);
                                            
                                            // Mark recording as active
                                            RECORDING_ACTIVE.store(true, Ordering::SeqCst);
                                            
                                            // Store capturer in state
                                            let mut cap = state.capturer.lock().unwrap();
                                            *cap = Some(capturer);
                                        }
                                        Err(e) => {
                                            error!("❌ Failed to start recording after pre-initialization: {}", e);
                                            let _ = events::emit_error(&app_clone, "CAPTURE_ERROR", &e);
                                            IS_RECORDING.store(false, Ordering::SeqCst);
                                            RECORDING_ACTIVE.store(false, Ordering::SeqCst);
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
                                }
                                Err(e) => {
                                    error!("❌ Failed to pre-initialize capturer for auto-wake recording: {}", e);
                                    let _ = events::emit_error(&app_clone, "CAPTURE_ERROR", &e);
                                    IS_RECORDING.store(false, Ordering::SeqCst);
                                    RECORDING_ACTIVE.store(false, Ordering::SeqCst);
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

/// Expands a path, replacing a leading ~/ with the user's home directory.
fn expand_home_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = crate::state::dirs::home_dir() {
            home.join(&path[2..]).to_string_lossy().to_string()
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    }
}

/// Open a folder in the system file explorer
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    // Expand ~ to home directory
    let expanded_path = expand_home_path(&path);
    
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
    let expanded_path = expand_home_path(&path);
    
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
    use log::{info, debug};
    
    info!("🔍 Starting get_projects");
    
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    debug!("📁 Output folder: {:?}", output_folder);

    let mut projects = Vec::new();

    if !output_folder.exists() {
        info!("📁 Output folder doesn't exist, returning empty list");
        return Ok(projects);
    }

    let entries = fs::read_dir(&output_folder).map_err(|e| format!("Failed to read output folder: {}", e))?;
    
    debug!("📂 Reading directory entries...");
    let mut dir_count = 0;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            dir_count += 1;
            let project_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            debug!("📁 Processing project directory: {}", project_name);

            // Check for timeline.json file
            let timeline_path = path.join("timeline.json");
            
            if timeline_path.exists() {
                debug!("📄 Found timeline.json for {}", project_name);
                // Read timeline to get project info
                match fs::read_to_string(&timeline_path) {
                    Ok(content) => {
                        debug!("📖 Read timeline content ({} bytes) for {}", content.len(), project_name);
                        match serde_json::from_str::<ProjectTimeline>(&content) {
                            Ok(timeline) => {
                                debug!("✅ Parsed timeline for {} ({} videos)", project_name, timeline.metadata.total_videos);
                                projects.push(Project {
                                    name: project_name,
                                    created_at: timeline.created_at,
                                    video_count: timeline.metadata.total_videos,
                                    last_modified: timeline.last_modified,
                                });
                            }
                            Err(e) => {
                                debug!("❌ Failed to parse timeline JSON for {}: {}", project_name, e);
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
                    Err(e) => {
                        debug!("❌ Failed to read timeline file for {}: {}", project_name, e);
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
                debug!("📄 No timeline.json found for {}, creating default entry", project_name);
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

    debug!("📋 Found {} project directories total", dir_count);
    
    // Sort projects by last modified date (newest first)
    debug!("🔄 Sorting {} projects by last modified date", projects.len());
    projects.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    info!("✅ Finished get_projects: {} projects returned", projects.len());
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
pub async fn set_current_project(project_name: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    use log::{info, debug};
    
    info!("🎯 Setting current project to: {}", project_name);
    let start_time = std::time::Instant::now();
    
    let output_folder = {
        let folder = state.output_folder.lock().map_err(|e| format!("Failed to lock output folder: {}", e))?;
        folder.clone()
    };

    let project_folder = output_folder.join(&project_name);

    if !project_folder.exists() {
        let elapsed = start_time.elapsed();
        debug!("❌ Project folder doesn't exist after {:.1}ms", elapsed.as_millis() as f32);
        return Err("Project does not exist".to_string());
    }

    {
        let mut current = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
        *current = Some(project_name.clone());
    }
    
    let project_set_time = start_time.elapsed();
    debug!("📝 Project state updated in {:.1}ms", project_set_time.as_millis() as f32);
    
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
    
    let capturer_create_time = start_time.elapsed();
    debug!("📱 Creating capturer after {:.1}ms", capturer_create_time.as_millis() as f32);
    
    let capturer_init_start = start_time.elapsed();
    debug!("⚡ Starting capturer pre-initialization in background after {:.1}ms (this may take 2-3 seconds)", capturer_init_start.as_millis() as f32);
    
    // Update status to initializing
    {
        if let Ok(mut status) = state.pre_init_status.lock() {
            *status = PreInitStatus::Initializing;
            let _ = events::emit_pre_init_status(&app, "Initializing");
        }
        // Update last activity
        if let Ok(mut activity) = state.last_activity.lock() {
            *activity = Instant::now();
        }
    }
    
    // Clone app handle for async block
    let app_clone = app.clone();
    
    // Pre-initialize in background (don't block UI)
    tauri::async_runtime::spawn(async move {
        // Create capturer inside the background task
        let mut capturer = ScreenCapturer::new(output_path, mic_enabled, audio_device_id);
        
        let bg_start = std::time::Instant::now();
        match capturer.pre_initialize(capture_region).await {
            Ok(()) => {
                let bg_elapsed = bg_start.elapsed();
                info!("✅ Capturer pre-initialized in background in {:.1}ms and ready for instant recording", bg_elapsed.as_millis() as f32);
                
                // Store the initialized capturer in state
                if let Some(app_state) = app.try_state::<AppState>() {
                    if let Ok(mut cap) = app_state.capturer.lock() {
                        *cap = Some(capturer);
                    } else {
                        warn!("⚠️  Failed to lock capturer state");
                    }
                    
                    // Update status to ready
                    if let Ok(mut status) = app_state.pre_init_status.lock() {
                        *status = PreInitStatus::Ready;
                        let _ = events::emit_pre_init_status(&app_clone, "Ready");
                    }
                } else {
                    warn!("⚠️  Failed to get app state");
                }
                
                // Clear initializing flag - ready to record
                IS_INITIALIZING.store(false, Ordering::SeqCst);
            }
            Err(e) => {
                let bg_elapsed = bg_start.elapsed();
                warn!("⚠️  Failed to pre-initialize capturer in background after {:.1}ms: {}", bg_elapsed.as_millis() as f32, e);
                
                // Update status back to not initialized on error
                if let Some(app_state) = app_clone.try_state::<AppState>() {
                    if let Ok(mut status) = app_state.pre_init_status.lock() {
                        *status = PreInitStatus::NotInitialized;
                        let _ = events::emit_pre_init_status(&app_clone, "NotInitialized");
                    }
                }
                
                IS_INITIALIZING.store(false, Ordering::SeqCst); // Clear flag on error
            }
        }
    });

    let total_time = start_time.elapsed();
    info!("🎯 set_current_project completed in {:.1}ms total (capturer initializing in background)", total_time.as_millis() as f32);
    Ok(())
}

/// Get the current project
#[tauri::command]
pub async fn get_current_project(state: State<'_, AppState>) -> Result<Option<String>, String> {
    use log::debug;
    
    debug!("🔍 Getting current project");
    let current = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
    debug!("📋 Current project: {:?}", current);
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
            
            // Calculate actual video duration
            let duration_ms = match calculate_video_duration(&file_path) {
                Ok(duration) => {
                    info!("📊 Calculated actual video duration: {}ms", duration);
                    duration
                },
                Err(e) => {
                    warn!("⚠️ Failed to calculate video duration, using file size estimate: {}", e);
                    estimate_duration_from_file_size(file_size)
                }
            };

            let entry = TimelineEntry {
                id: entry_id,
                filename: filename.clone(),
                recorded_at: created_rfc3339,
                duration_ms,
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

    // Only save timeline if changes were detected (avoid infinite filesystem watcher loop)
    if changes_count > 0 {
        let timeline_json = serde_json::to_string_pretty(&timeline)
            .map_err(|e| format!("Failed to serialize timeline: {}", e))?;

        fs::write(&timeline_path, timeline_json)
            .map_err(|e| format!("Failed to write timeline.json: {}", e))?;
        
        info!("✅ Reconciliation complete: {} changes detected and saved", changes_count);
    } else {
        debug!("✅ Reconciliation complete: 0 changes detected (no file write needed)");
    }
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

// Helper function to calculate actual video duration from file
#[cfg(target_os = "macos")]
fn calculate_video_duration(file_path: &std::path::Path) -> Result<u64, String> {
    use std::process::Command;
    
    // Use ffprobe to get actual video duration
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            file_path.to_str().ok_or("Invalid file path")?
        ])
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;
    
    if !output.status.success() {
        return Err("ffprobe failed to read video duration".to_string());
    }
    
    let duration_str = String::from_utf8_lossy(&output.stdout);
    let duration_seconds: f64 = duration_str.trim().parse()
        .map_err(|e| format!("Failed to parse duration: {}", e))?;
    
    Ok((duration_seconds * 1000.0) as u64) // Convert to milliseconds
}

#[cfg(not(target_os = "macos"))]
fn calculate_video_duration(file_path: &std::path::Path) -> Result<u64, String> {
    // Fallback for non-macOS platforms
    estimate_duration_from_file_size(std::fs::metadata(file_path)?.len())
}

// Helper function to estimate duration from file size (fallback)
fn estimate_duration_from_file_size(file_size: u64) -> u64 {
    // Very rough estimate: assume ~1MB per second for typical screen recordings
    let estimated_seconds = (file_size / (1024 * 1024)).max(1); // At least 1 second
    estimated_seconds * 1000 // Convert to milliseconds
}

/// Add a recording entry to the current project's timeline
/// This is the proper place for duration management - the timeline system handles
/// all duration calculations, storage, and metadata management.
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

/// Get the current pre-initialization status
#[tauri::command]
pub async fn get_pre_init_status(state: State<'_, AppState>) -> Result<String, String> {
    let status = state.pre_init_status.lock().map_err(|e| format!("Failed to lock pre_init_status: {}", e))?;
    let status_str = match *status {
        PreInitStatus::NotInitialized => "not_initialized",
        PreInitStatus::Initializing => "initializing", 
        PreInitStatus::Ready => "ready",
        PreInitStatus::ShuttingDown => "shutting_down",
    };
    Ok(status_str.to_string())
}

/// Get the idle timeout setting in minutes
#[tauri::command]
pub async fn get_idle_timeout_mins(state: State<'_, AppState>) -> Result<u32, String> {
    let timeout = state.idle_timeout_mins.lock().map_err(|e| format!("Failed to lock idle_timeout_mins: {}", e))?;
    Ok(*timeout)
}

/// Set the idle timeout setting in minutes
#[tauri::command]
pub async fn set_idle_timeout_mins(timeout_mins: u32, state: State<'_, AppState>) -> Result<(), String> {
    let mut timeout = state.idle_timeout_mins.lock().map_err(|e| format!("Failed to lock idle_timeout_mins: {}", e))?;
    *timeout = timeout_mins;
    info!("⏰ Idle timeout set to {} minutes", timeout_mins);
    Ok(())
}

/// Update last activity timestamp (called on user interactions)
#[tauri::command] 
pub async fn update_activity(state: State<'_, AppState>) -> Result<(), String> {
    let mut activity = state.last_activity.lock().map_err(|e| format!("Failed to lock last_activity: {}", e))?;
    *activity = Instant::now();
    debug!("📱 User activity updated");
    Ok(())
}

/// Shutdown capturer due to idle timeout
#[tauri::command]
pub async fn shutdown_idle_capturer(state: State<'_, AppState>) -> Result<(), String> {
    info!("💤 Shutting down capturer due to idle timeout");
    
    // Update status to shutting down
    {
        let mut status = state.pre_init_status.lock().map_err(|e| format!("Failed to lock pre_init_status: {}", e))?;
        *status = PreInitStatus::ShuttingDown;
        // Note: emit_pre_init_status needs AppHandle, but this command doesn't have it
        // Status change will be detected by frontend polling or by idle checker
    }
    
    // Clear the capturer
    {
        let mut capturer = state.capturer.lock().map_err(|e| format!("Failed to lock capturer: {}", e))?;
        *capturer = None;
    }
    
    // Update status to not initialized
    {
        let mut status = state.pre_init_status.lock().map_err(|e| format!("Failed to lock pre_init_status: {}", e))?;
        *status = PreInitStatus::NotInitialized;
        // Note: emit_pre_init_status needs AppHandle, but this command doesn't have it
        // Status change will be detected by frontend polling or by idle checker
    }
    
    // Clear initializing flag
    IS_INITIALIZING.store(false, Ordering::SeqCst);
    
    info!("✅ Capturer shutdown complete");
    Ok(())
}

/// Start the idle timeout checker background task
pub fn start_idle_checker(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Check every minute
        
        loop {
            interval.tick().await;
            
            if let Some(state) = app_handle.try_state::<AppState>() {
                // Check if we should shut down due to idle timeout
                let should_shutdown = {
                    let timeout_result = state.idle_timeout_mins.lock();
                    let activity_result = state.last_activity.lock();
                    let status_result = state.pre_init_status.lock();
                    
                    match (timeout_result, activity_result, status_result) {
                        (Ok(timeout_mins), Ok(last_activity), Ok(status)) => {
                            if *timeout_mins == 0 {
                                false // Timeout disabled
                            } else {
                                let timeout_duration = Duration::from_secs((*timeout_mins as u64) * 60);
                                let idle_time = last_activity.elapsed();
                                
                                idle_time > timeout_duration && matches!(*status, PreInitStatus::Ready)
                            }
                        }
                        _ => false
                    }
                };
                
                if should_shutdown {
                    info!("💤 Idle timeout reached, shutting down pre-initialized capturer");
                    let _ = events::emit_pre_init_status(&app_handle, "ShuttingDown");
                    let _ = app_handle.emit_to("main", "pre-init-idle-shutdown", ());
                    
                    // Call shutdown command
                    if let Err(e) = shutdown_idle_capturer(app_handle.state::<AppState>()).await {
                        warn!("Failed to shutdown idle capturer: {}", e);
                    }
                    let _ = events::emit_pre_init_status(&app_handle, "NotInitialized");
                }
            }
        }
    });
    
    info!("⏰ Idle timeout checker started");
}

/// Toggle pre-initialization state (manual control)
#[tauri::command]
pub async fn toggle_pre_init(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    info!("🔄 Manual toggle of pre-initialization requested");
    
    // Update activity first
    if let Ok(mut activity) = state.last_activity.lock() {
        *activity = Instant::now();
    }
    
    // Check current status
    let current_status = {
        let status = state.pre_init_status.lock().map_err(|e| format!("Failed to lock pre_init_status: {}", e))?;
        status.clone()
    };
    
    match current_status {
        PreInitStatus::NotInitialized => {
            info!("🚀 Starting manual pre-initialization");
            
            // Get current project
            let project_name = {
                let current = state.current_project.lock().map_err(|e| format!("Failed to lock current project: {}", e))?;
                current.as_ref().ok_or("No current project set")?.clone()
            };
            
            // Trigger pre-initialization using set_current_project logic
            let _ = set_current_project(project_name, app, state).await;
            Ok("Initializing".to_string())
        }
        PreInitStatus::Initializing => {
            info!("⚠️  Pre-initialization already in progress");
            Ok("Initializing".to_string())
        }
        PreInitStatus::Ready => {
            info!("💤 Shutting down pre-initialized capturer (manual)");
            let _ = shutdown_idle_capturer(state).await?;
            let _ = events::emit_pre_init_status(&app, "NotInitialized");
            Ok("NotInitialized".to_string())
        }
        PreInitStatus::ShuttingDown => {
            info!("⚠️  Capturer is already shutting down");
            Ok("ShuttingDown".to_string())
        }
    }
}

/// Handle window focus gained event
#[tauri::command]
pub async fn on_window_focus_gained(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    info!("👁️  Window gained focus");
    
    // Update focus state and get previous state
    let was_focused = {
        let mut focused = state.window_focused.lock().map_err(|e| format!("Failed to lock window_focused: {}", e))?;
        let was_focused = *focused;
        *focused = true;
        was_focused
    }; // Lock is released here
    
    // Only restart pre-initialization if window was previously unfocused
    if !was_focused {
        info!("🔄 Window came back into view - restarting pre-initialization");
        
        // Update activity timestamp
        if let Ok(mut activity) = state.last_activity.lock() {
            *activity = Instant::now();
        }
        
        // Check if we have a current project and restart pre-initialization
        let current_project = {
            let project = state.current_project.lock().map_err(|e| format!("Failed to lock current_project: {}", e))?;
            project.clone()
        };
        
        if let Some(project_name) = current_project {
            // Check current pre-init status
            let current_status = {
                let status = state.pre_init_status.lock().map_err(|e| format!("Failed to lock pre_init_status: {}", e))?;
                status.clone()
            };
            
            // Only restart if not already initializing or ready
            match current_status {
                PreInitStatus::NotInitialized | PreInitStatus::ShuttingDown => {
                    info!("🚀 Restarting pre-initialization for project: {}", project_name);
                    // Trigger pre-initialization by setting current project again
                    let _ = set_current_project(project_name, app, state).await;
                }
                PreInitStatus::Initializing => {
                    info!("⚡ Pre-initialization already in progress, no restart needed");
                }
                PreInitStatus::Ready => {
                    info!("✅ Pre-initialization already ready, no restart needed");
                }
            }
        } else {
            info!("📋 No current project set, skipping pre-initialization restart");
        }
    }
    
    Ok(())
}

/// Handle window focus lost event
#[tauri::command]
pub async fn on_window_focus_lost(state: State<'_, AppState>) -> Result<(), String> {
    info!("👁️  Window lost focus");
    
    // Update focus state
    {
        let mut focused = state.window_focused.lock().map_err(|e| format!("Failed to lock window_focused: {}", e))?;
        *focused = false;
    }
    
    Ok(())
}

/// Setup window focus event listeners
pub fn setup_window_focus_listeners(app_handle: AppHandle) {
    use tauri::WindowEvent;
    
    if let Some(window) = app_handle.get_webview_window("main") {
        window.on_window_event(move |event| {
            match event {
                WindowEvent::Focused(focused) => {
                    let app_clone = app_handle.clone();
                    
                    if *focused {
                        // Window gained focus - handle sync
                        handle_window_focus_gained_sync(app_clone);
                    } else {
                        // Window lost focus - handle sync
                        handle_window_focus_lost_sync(app_clone);
                    }
                }
                _ => {}
            }
        });
        
        info!("✅ Window focus event listeners setup complete");
    } else {
        warn!("⚠️  Main window not found for focus event setup");
    }
}

/// Handle window focus gained synchronously (to avoid async issues in event callback)
fn handle_window_focus_gained_sync(app_handle: AppHandle) {
    let state = app_handle.state::<AppState>();
    
    // Update focus state and get previous state
    let was_focused = {
        if let Ok(mut focused) = state.window_focused.lock() {
            let was_focused = *focused;
            *focused = true;
            was_focused
        } else {
            return; // Failed to lock, skip processing
        }
    };
    
    if !was_focused {
        info!("🔄 Window came back into view - scheduling pre-initialization restart");
        
        // Update activity timestamp
        if let Ok(mut activity) = state.last_activity.lock() {
            *activity = Instant::now();
        }
        
        // Spawn async task to handle pre-initialization restart
        let app_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_clone.state::<AppState>();
            
            // Check if we have a current project and restart pre-initialization
            let current_project = {
                if let Ok(project) = state.current_project.lock() {
                    project.clone()
                } else {
                    return; // Failed to lock
                }
            };
            
            if let Some(project_name) = current_project {
                // Check current pre-init status
                let current_status = {
                    if let Ok(status) = state.pre_init_status.lock() {
                        status.clone()
                    } else {
                        return; // Failed to lock
                    }
                };
                
                // Only restart if not already initializing or ready
                match current_status {
                    PreInitStatus::NotInitialized | PreInitStatus::ShuttingDown => {
                        info!("🚀 Restarting pre-initialization for project: {}", project_name);
                        // Trigger pre-initialization by setting current project again
                        if let Err(e) = set_current_project(project_name, app_clone.clone(), state).await {
                            warn!("Failed to restart pre-initialization: {}", e);
                        }
                    }
                    PreInitStatus::Initializing => {
                        info!("⚡ Pre-initialization already in progress, no restart needed");
                    }
                    PreInitStatus::Ready => {
                        info!("✅ Pre-initialization already ready, no restart needed");
                    }
                }
            } else {
                info!("📋 No current project set, skipping pre-initialization restart");
            }
        });
    }
}

/// Handle window focus lost synchronously
fn handle_window_focus_lost_sync(app_handle: AppHandle) {
    info!("👁️  Window lost focus");
    
    let state = app_handle.state::<AppState>();
    
    // Update focus state
    if let Ok(mut focused) = state.window_focused.lock() {
        *focused = false;
    }; // Add semicolon to drop temporaries sooner
}
