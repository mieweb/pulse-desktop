use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use crate::fs_watcher::WatcherControl;

#[cfg(target_os = "macos")]
use crate::capture::macos::ScreenCapturer;

#[cfg(target_os = "windows")]
use crate::capture::windows::ScreenCapturer;

/// Pre-initialization status for capturer
#[derive(Debug, Clone, PartialEq)]
pub enum PreInitStatus {
    NotInitialized,
    Initializing,
    Ready,
    ShuttingDown,
}

/// Application state for managing recording settings
pub struct AppState {
    pub output_folder: Mutex<PathBuf>,
    pub mic_enabled: Mutex<bool>,
    pub selected_audio_device: Mutex<Option<String>>, // Audio device ID
    pub clip_count: Mutex<u32>,
    /// Recording state tracked internally (actual state managed via atomic bools in commands.rs)
    #[allow(dead_code)]
    pub is_recording: Mutex<bool>,
    pub capturer: Mutex<Option<ScreenCapturer>>,
    pub capture_region: Mutex<Option<(u32, u32, u32, u32)>>, // x, y, width, height
    pub current_project: Mutex<Option<String>>,
    pub watcher_control: Mutex<Option<WatcherControl>>,
    
    // Pre-initialization state tracking
    pub pre_init_status: Mutex<PreInitStatus>,
    pub last_activity: Mutex<Instant>,
    pub idle_timeout_mins: Mutex<u32>, // Default to 30 minutes
    
    // Window focus tracking
    pub window_focused: Mutex<bool>, // Track if window is currently focused
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
            
            // Initialize pre-init state tracking
            pre_init_status: Mutex::new(PreInitStatus::NotInitialized),
            last_activity: Mutex::new(Instant::now()),
            idle_timeout_mins: Mutex::new(5), // 5 mins
            
            // Initialize window focus tracking
            window_focused: Mutex::new(true), // Assume focused on startup
        }
    }
}

/// Module for directory operations
pub mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}
