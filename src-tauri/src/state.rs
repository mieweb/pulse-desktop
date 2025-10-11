use std::path::PathBuf;
use std::sync::Mutex;

/// Application state for managing recording settings
pub struct AppState {
    pub output_folder: Mutex<PathBuf>,
    pub mic_enabled: Mutex<bool>,
    pub clip_count: Mutex<u32>,
    pub is_recording: Mutex<bool>,
}

impl AppState {
    pub fn new() -> Self {
        // Default output folder based on platform
        #[cfg(target_os = "macos")]
        let default_folder = dirs::home_dir()
            .map(|p| p.join("Movies").join("PushToHold"))
            .unwrap_or_else(|| PathBuf::from("~/Movies/PushToHold"));

        #[cfg(target_os = "windows")]
        let default_folder = dirs::home_dir()
            .map(|p| p.join("Videos").join("PushToHold"))
            .unwrap_or_else(|| PathBuf::from("~/Videos/PushToHold"));

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let default_folder = PathBuf::from("~/Videos/PushToHold");

        Self {
            output_folder: Mutex::new(default_folder),
            mic_enabled: Mutex::new(true),
            clip_count: Mutex::new(0),
            is_recording: Mutex::new(false),
        }
    }
}

/// Module for directory operations
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}
