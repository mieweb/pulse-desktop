use notify::{Watcher, RecursiveMode, Result as NotifyResult, Event, EventKind};
use std::path::PathBuf;
use std::sync::{mpsc::channel, Arc, atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Emitter};
use log::{debug, info};

/// Control handle for the filesystem watcher
pub struct WatcherControl {
    enabled: Arc<AtomicBool>,
}

impl WatcherControl {
    /// Pause filesystem event emission (during recording)
    pub fn pause(&self) {
        info!("⏸️  Pausing filesystem watcher");
        self.enabled.store(false, Ordering::SeqCst);
    }
    
    /// Resume filesystem event emission (after recording completes)
    pub fn resume(&self) {
        info!("▶️  Resuming filesystem watcher");
        self.enabled.store(true, Ordering::SeqCst);
    }
    
    /// Check if watcher is currently enabled
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
}

/// Start watching the output folder for filesystem changes
/// Returns a WatcherControl handle to pause/resume event emission
pub fn watch_output_folder(app: AppHandle, output_folder: PathBuf) -> NotifyResult<WatcherControl> {
    info!("📁 Starting filesystem watcher for: {:?}", output_folder);
    
    let (tx, rx) = channel();
    
    // Create filesystem watcher
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })?;
    
    // Watch the output folder recursively
    watcher.watch(&output_folder, RecursiveMode::Recursive)?;
    
    // Create control handle
    let enabled = Arc::new(AtomicBool::new(true));
    let enabled_clone = enabled.clone();
    
    // Spawn background thread to handle filesystem events
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            // Check if watcher is enabled
            if !enabled_clone.load(Ordering::SeqCst) {
                debug!("📂 Filesystem event received but PAUSED: {:?}", event);
                continue;
            }
            
            // Log all events for debugging
            debug!("📂 Filesystem event received: {:?}", event);
            
            match event.kind {
                EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                    // Check if any of the changed paths are video files or directories
                    for path in &event.paths {
                        debug!("   Path changed: {:?}", path);
                        
                        // Check if it's a video file
                        if is_video_file(path) {
                            debug!("   ✅ Video file change detected: {:?}", path.file_name().unwrap_or_default());
                            let _ = app.emit("filesystem-changed", ());
                            break;
                        }
                        
                        // Check if it's a directory (project folder changes)
                        if path.is_dir() {
                            debug!("   ✅ Directory change detected: {:?}", path.file_name().unwrap_or_default());
                            let _ = app.emit("filesystem-changed", ());
                            break;
                        }
                        
                        debug!("   ⏭️  Not a video file or directory, skipping");
                    }
                }
                _ => {
                    debug!("   ⏭️  Ignored event type: {:?}", event.kind);
                }
            }
        }
    });
    
    // Keep watcher alive by leaking it
    // This is acceptable because we want it to run for the lifetime of the app
    Box::leak(Box::new(watcher));
    
    info!("✅ Filesystem watcher started successfully");
    Ok(WatcherControl { enabled })
}

/// Check if a path is a video file based on extension
fn is_video_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(ext_str.as_str(), "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v")
    } else {
        false
    }
}
