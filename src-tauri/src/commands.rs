use tauri::{State, AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::state::AppState;
use crate::events;

#[derive(serde::Serialize)]
pub struct RecordingInfo {
    pub filename: String,
    pub path: String,
    pub size: u64,
    pub created: u64,
    pub thumbnail_path: Option<String>,
}

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
                                    
                                    // Generate thumbnail automatically
                                    let video_path = path.to_string_lossy().to_string();
                                    let thumbnail_result = generate_thumbnail_internal(&video_path);
                                    match thumbnail_result {
                                        Ok(thumbnail_path) => {
                                            println!("🖼️ Auto-generated thumbnail: {}", thumbnail_path);
                                            // Small delay to ensure file is fully written
                                            std::thread::sleep(std::time::Duration::from_millis(100));
                                        }
                                        Err(e) => {
                                            println!("⚠️ Failed to auto-generate thumbnail: {}", e);
                                        }
                                    }
                                    
                                    // Emit clip saved event AFTER thumbnail generation
                                    let _ = events::emit_clip_saved(&app_clone, events::ClipSavedEvent {
                                        path: video_path,
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

/// List all recordings in the output folder
#[tauri::command]
pub fn list_recordings(state: State<AppState>) -> Result<Vec<RecordingInfo>, String> {
    let folder = state.output_folder.lock().unwrap().clone();
    
    println!("🔍 Scanning folder: {:?}", folder);
    
    if !folder.exists() {
        println!("❌ Output folder does not exist: {:?}", folder);
        return Ok(vec![]);
    }
    
    let mut recordings = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&folder) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                println!("🔍 Found file: {}", file_name);
                if file_name.ends_with(".mp4") {
                    // Check if the filename (without extension) is purely numeric
                    let name_without_ext = file_name.trim_end_matches(".mp4");
                    if name_without_ext.chars().all(|c| c.is_ascii_digit()) {
                        println!("✅ Valid recording file: {}", file_name);
                    if let Ok(metadata) = entry.metadata() {
                        let path = entry.path().to_string_lossy().to_string();
                        
                        // Try new naming pattern first (1_thumb.jpg)
                        let thumbnail_path = entry.path().with_extension("").to_string_lossy().to_string() + "_thumb.jpg";
                        let thumbnail_exists = std::path::Path::new(&thumbnail_path).exists();
                        println!("🔍 Checking thumbnail: {} (exists: {})", thumbnail_path, thumbnail_exists);
                        
                        // If new pattern doesn't exist, try old pattern (recording-1_thumb.jpg)
                        let final_thumbnail_path = if thumbnail_exists {
                            println!("✅ Using new thumbnail: {}", thumbnail_path);
                            Some(thumbnail_path)
                        } else {
                            let old_thumbnail_path = entry.path().parent()
                                .unwrap_or(&entry.path())
                                .join(format!("recording-{}_thumb.jpg", name_without_ext));
                            println!("🔍 Checking old thumbnail: {} (exists: {})", old_thumbnail_path.display(), old_thumbnail_path.exists());
                            if old_thumbnail_path.exists() {
                                println!("✅ Using old thumbnail: {}", old_thumbnail_path.display());
                                Some(old_thumbnail_path.to_string_lossy().to_string())
                            } else {
                                println!("❌ No thumbnail found for {}", file_name);
                                None
                            }
                        };
                        
                        recordings.push(RecordingInfo {
                            filename: file_name.to_string(),
                            path,
                            size: metadata.len(),
                            created: metadata.created()
                                .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            thumbnail_path: final_thumbnail_path,
                        });
                    }
                    } else {
                        println!("❌ Invalid file (not numeric): {}", file_name);
                    }
                } else {
                    println!("❌ Not an MP4 file: {}", file_name);
                }
            }
        }
    }
    
    // Sort by creation time (oldest first)
    recordings.sort_by(|a, b| a.created.cmp(&b.created));
    
    // Update clip count
    {
        let mut count = state.clip_count.lock().unwrap();
        *count = recordings.len() as u32;
    }
    
    println!("📊 Found {} recordings", recordings.len());
    Ok(recordings)
}

/// Delete a recording file
#[tauri::command]
pub fn delete_recording(filename: String, state: State<AppState>) -> Result<(), String> {
    let folder = state.output_folder.lock().unwrap().clone();
    let file_path = folder.join(&filename);
    
    // Validate the path is within the output folder
    if !file_path.starts_with(&folder) {
        return Err("Invalid file path".to_string());
    }
    
    if file_path.exists() {
        std::fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;
        
        // Also delete thumbnail if it exists
        let thumbnail_path = file_path.with_extension("").to_string_lossy().to_string() + "_thumb.jpg";
        if std::path::Path::new(&thumbnail_path).exists() {
            let _ = std::fs::remove_file(&thumbnail_path);
        }
        
        // Renumber remaining recordings
        renumber_recordings(&folder)?;
        
        // Update clip count
        {
            let mut count = state.clip_count.lock().unwrap();
            *count = count.saturating_sub(1);
        }
        
        println!("🗑️ Deleted recording: {}", filename);
    }
    
    Ok(())
}

/// Renumber recordings sequentially after deletion
fn renumber_recordings(folder: &std::path::Path) -> Result<(), String> {
    let mut recordings = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".mp4") {
                    // Check if the filename (without extension) is purely numeric
                    let name_without_ext = file_name.trim_end_matches(".mp4");
                    if name_without_ext.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(metadata) = entry.metadata() {
                            recordings.push((entry.path(), metadata.created().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }
        }
    }
    
    // Sort by creation time (oldest first)
    recordings.sort_by(|a, b| a.1.cmp(&b.1));
    
    // Rename files sequentially
    for (i, (old_path, _)) in recordings.iter().enumerate() {
        let new_name = format!("{}.mp4", i + 1);
        let new_path = folder.join(&new_name);
        
        if old_path != &new_path {
            std::fs::rename(old_path, &new_path)
                .map_err(|e| format!("Failed to rename file: {}", e))?;
            
            // Also rename thumbnail if it exists
            let old_thumb = old_path.with_extension("").to_string_lossy().to_string() + "_thumb.jpg";
            let new_thumb = new_path.with_extension("").to_string_lossy().to_string() + "_thumb.jpg";
            if std::path::Path::new(&old_thumb).exists() {
                let _ = std::fs::rename(&old_thumb, &new_thumb);
            }
        }
    }
    
    Ok(())
}

/// Generate thumbnail for a recording using FFmpeg command
#[tauri::command]
pub fn generate_thumbnail(video_path: String) -> Result<String, String> {
    generate_thumbnail_internal(&video_path)
}

/// Internal thumbnail generation function (used automatically after recording)
fn generate_thumbnail_internal(video_path: &str) -> Result<String, String> {
    let video_file = std::path::PathBuf::from(&video_path);

    if !video_file.exists() {
        return Err("Video file does not exist".to_string());
    }

    // Create thumbnail filename
    let thumbnail_name = video_file.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{}_thumb.jpg", s))
        .ok_or("Invalid video filename")?;

    let thumbnail_path = video_file.parent()
        .ok_or("Invalid video path")?
        .join(&thumbnail_name);

    // First, get the video duration to calculate center frame
    let duration_output = std::process::Command::new("ffprobe")
        .args(&[
            "-v", "quiet",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            &video_path
        ])
        .output();

    let center_time = match duration_output {
        Ok(result) => {
            if result.status.success() {
                let duration_str = String::from_utf8_lossy(&result.stdout);
                let duration_str = duration_str.trim();
                if let Ok(duration) = duration_str.parse::<f64>() {
                    let center = duration / 2.0;
                    println!("📹 Video duration: {:.2}s, using center at {:.2}s", duration, center);
                    center.to_string()
                } else {
                    "0.1".to_string() // Fallback to 0.1s if duration parsing fails
                }
            } else {
                "0.1".to_string() // Fallback to 0.1s if ffprobe fails
            }
        }
        Err(_) => "0.1".to_string() // Fallback to 0.1s if ffprobe not available
    };

    // Use center frame for thumbnail
    let output = std::process::Command::new("ffmpeg")
        .args(&[
            "-i", &video_path,
            "-ss", &center_time,  // Seek to center of video
            "-vframes", "1",  // Extract exactly 1 frame
            "-vf", "scale=320:240",  // Simple scale to thumbnail size
            "-f", "image2",  // Output format
            "-y",  // Overwrite if exists
            &thumbnail_path.to_string_lossy()
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("🖼️ Generated thumbnail: {}", thumbnail_path.display());
                Ok(thumbnail_path.to_string_lossy().to_string())
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                println!("⚠️ FFmpeg failed: {}", error);
                
                // Try with even simpler command as fallback (still using center)
                let fallback_output = std::process::Command::new("ffmpeg")
                    .args(&[
                        "-i", &video_path,
                        "-ss", &center_time,  // Still use center frame
                        "-vframes", "1",  // Just 1 frame
                        "-s", "320x240",  // Simple size specification
                        "-y",  // Overwrite
                        &thumbnail_path.to_string_lossy()
                    ])
                    .output();
                
                match fallback_output {
                    Ok(fallback_result) => {
                        if fallback_result.status.success() {
                            println!("🖼️ Generated thumbnail (fallback): {}", thumbnail_path.display());
                            Ok(thumbnail_path.to_string_lossy().to_string())
                        } else {
                            let fallback_error = String::from_utf8_lossy(&fallback_result.stderr);
                            Err(format!("FFmpeg failed with both methods. Error: {}", fallback_error))
                        }
                    }
                    Err(e) => {
                        Err(format!("Failed to run FFmpeg: {}. Make sure FFmpeg is installed and in PATH.", e))
                    }
                }
            }
        }
        Err(e) => {
            Err(format!("Failed to run FFmpeg: {}. Make sure FFmpeg is installed and in PATH.", e))
        }
    }
}

/// Read thumbnail file and return as base64
#[tauri::command]
pub fn read_thumbnail_file(file_path: String) -> Result<String, String> {
    use std::fs;
    use base64::{Engine as _, engine::general_purpose};
    
    let file_data = fs::read(&file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;
    
    let base64_data = general_purpose::STANDARD.encode(&file_data);
    Ok(base64_data)
}

/// Play a recording file using the system default player
#[tauri::command]
pub fn play_recording(file_path: String) -> Result<(), String> {
    use std::process::Command;
    
    #[cfg(target_os = "macos")]
    {
        let result = Command::new("open")
            .arg(&file_path)
            .spawn();
        
        match result {
            Ok(_) => {
                println!("🎬 Playing recording: {}", file_path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to play recording: {}", e))
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        let result = Command::new("cmd")
            .args(&["/C", "start", "", &file_path])
            .spawn();
        
        match result {
            Ok(_) => {
                println!("🎬 Playing recording: {}", file_path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to play recording: {}", e))
        }
    }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let result = Command::new("xdg-open")
            .arg(&file_path)
            .spawn();
        
        match result {
            Ok(_) => {
                println!("🎬 Playing recording: {}", file_path);
                Ok(())
            }
            Err(e) => Err(format!("Failed to play recording: {}", e))
        }
    }
}

/// Reorder recordings based on new order
#[tauri::command]
pub fn reorder_recordings(new_order: Vec<String>, state: State<AppState>) -> Result<(), String> {
    let folder = state.output_folder.lock().unwrap().clone();
    
    if !folder.exists() {
        return Err("Output folder does not exist".to_string());
    }
    
    // Get all current recordings with their metadata
    let mut recordings = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&folder) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".mp4") {
                    // Check if the filename (without extension) is purely numeric
                    let name_without_ext = file_name.trim_end_matches(".mp4");
                    if name_without_ext.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(metadata) = entry.metadata() {
                            recordings.push((entry.path(), metadata.created().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }
        }
    }
    
    // Sort by creation time to get the original order
    recordings.sort_by(|a, b| a.1.cmp(&b.1));
    
    // Validate that new_order contains all recordings
    if new_order.len() != recordings.len() {
        return Err("Invalid reorder: number of items doesn't match".to_string());
    }
    
    // Create temporary names to avoid conflicts during renaming
    let temp_prefix = "temp_recording_";
    let mut temp_files = Vec::new();
    
    // First, rename all files to temporary names
    for (i, (old_path, _)) in recordings.iter().enumerate() {
        let temp_name = format!("{}{}.mp4", temp_prefix, i);
        let temp_path = folder.join(&temp_name);
        
        std::fs::rename(old_path, &temp_path)
            .map_err(|e| format!("Failed to rename to temp: {}", e))?;
        
        // Also rename thumbnail if it exists
        let old_thumb = old_path.with_extension("").to_string_lossy().to_string() + "_thumb.jpg";
        let temp_thumb = temp_path.with_extension("").to_string_lossy().to_string() + "_thumb.jpg";
        if std::path::Path::new(&old_thumb).exists() {
            let _ = std::fs::rename(&old_thumb, &temp_thumb);
        }
        
        temp_files.push((temp_path, temp_thumb));
    }
    
    // Now rename from temp names to final names based on new order
    for (i, _filename) in new_order.iter().enumerate() {
        let final_name = format!("{}.mp4", i + 1);
        let final_path = folder.join(&final_name);
        let final_thumb = folder.join(&format!("{}_thumb.jpg", i + 1));
        
        // Find the temp file that corresponds to this filename
        if let Some((temp_path, temp_thumb)) = temp_files.get(i) {
            std::fs::rename(temp_path, &final_path)
                .map_err(|e| format!("Failed to rename to final: {}", e))?;
            
            // Rename thumbnail if it exists
            if std::path::Path::new(temp_thumb).exists() {
                let _ = std::fs::rename(temp_thumb, &final_thumb);
            }
        }
    }
    
    println!("🔄 Reordered {} recordings", recordings.len());
    Ok(())
}
