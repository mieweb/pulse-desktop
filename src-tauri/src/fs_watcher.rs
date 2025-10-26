use notify::{Watcher, RecursiveMode, Result as NotifyResult, Event, EventKind};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use tauri::{AppHandle, Emitter};

/// Start watching the output folder for filesystem changes
pub fn watch_output_folder(app: AppHandle, output_folder: PathBuf) -> NotifyResult<()> {
    println!("📁 Starting filesystem watcher for: {:?}", output_folder);
    
    let (tx, rx) = channel();
    
    // Create filesystem watcher
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })?;
    
    // Watch the output folder recursively
    watcher.watch(&output_folder, RecursiveMode::Recursive)?;
    
    // Spawn background thread to handle filesystem events
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            // Log all events for debugging
            println!("📂 Filesystem event received: {:?}", event);
            
            match event.kind {
                EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                    // Check if any of the changed paths are video files or directories
                    for path in &event.paths {
                        println!("   Path changed: {:?}", path);
                        
                        // Check if it's a video file
                        if is_video_file(path) {
                            println!("   ✅ Video file change detected: {:?}", path.file_name().unwrap_or_default());
                            let _ = app.emit("filesystem-changed", ());
                            break;
                        }
                        
                        // Check if it's a directory (project folder changes)
                        if path.is_dir() {
                            println!("   ✅ Directory change detected: {:?}", path.file_name().unwrap_or_default());
                            let _ = app.emit("filesystem-changed", ());
                            break;
                        }
                        
                        println!("   ⏭️  Not a video file or directory, skipping");
                    }
                }
                _ => {
                    println!("   ⏭️  Ignored event type: {:?}", event.kind);
                }
            }
        }
    });
    
    // Keep watcher alive by leaking it
    // This is acceptable because we want it to run for the lifetime of the app
    Box::leak(Box::new(watcher));
    
    println!("✅ Filesystem watcher started successfully");
    Ok(())
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
