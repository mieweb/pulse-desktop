// Hotkey module - global hotkey registration and handling

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[allow(unused)]
#[cfg(target_os = "macos")]
mod platform {
    pub use super::macos::*;
}

#[allow(unused)]
#[cfg(target_os = "windows")]
mod platform {
    pub use super::windows::*;
}


pub type HotkeyCallback = Box<dyn Fn(bool) + Send + 'static>;

/// Global hotkey management trait
/// Reserved for future hotkey feature implementation (currently using DebugControls in UI)
#[allow(dead_code)]
pub trait HotkeyManager {
    fn register(&mut self, callback: HotkeyCallback) -> Result<(), String>;
    fn unregister(&mut self) -> Result<(), String>;
}

