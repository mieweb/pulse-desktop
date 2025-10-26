// Capture module - platform-specific screen capture implementations

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
