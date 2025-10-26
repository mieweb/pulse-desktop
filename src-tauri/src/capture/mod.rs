// Capture module - platform-specific screen capture implementations

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

// Platform abstraction module (not currently used but part of design pattern)
#[allow(unused)]
#[cfg(target_os = "macos")]
mod platform {
    pub use super::macos::*;
}

// Platform abstraction module (not currently used but part of design pattern)
#[allow(unused)]
#[cfg(target_os = "windows")]
mod platform {
    pub use super::windows::*;
}
