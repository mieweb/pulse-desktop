use std::path::PathBuf;
use std::sync::Mutex;
use crate::fs_watcher::WatcherControl;

#[cfg(target_os = "macos")]
use crate::capture::macos::ScreenCapturer;

#[cfg(target_os = "windows")]
use crate::capture::windows::ScreenCapturer;

/// Application state for managing recording settings
pub struct AppState {
    pub output_folder: Mutex<PathBuf>,
    pub mic_enabled: Mutex<bool>,
    pub selected_audio_device: Mutex<Option<String>>, // Audio device ID
    pub clip_count: Mutex<u32>,
    #[allow(dead_code)]
    pub is_recording: Mutex<bool>,
    pub capturer: Mutex<Option<ScreenCapturer>>,
    pub capture_region: Mutex<Option<(u32, u32, u32, u32)>>, // x, y, width, height
    pub current_project: Mutex<Option<String>>,
    pub watcher_control: Mutex<Option<WatcherControl>>,
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
            selected_audio_device: Mutex::new(None), // Auto-select on first use
            clip_count: Mutex::new(0),
            is_recording: Mutex::new(false),
            capturer: Mutex::new(None),
            capture_region: Mutex::new(None), // Start with full screen (no region)
            current_project: Mutex::new(None),
            watcher_control: Mutex::new(None),
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
