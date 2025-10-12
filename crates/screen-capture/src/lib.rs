// Cross-platform native video screen capture library
//! 
//! # screen-capture
//! 
//! Native, efficient screen recording to MP4 using platform APIs:
//! - macOS: ScreenCaptureKit + AVAssetWriter
//! - Windows: Desktop Duplication API + Media Foundation
//! 
//! ## Features
//! - Direct MP4 encoding (no transcoding)
//! - Hardware-accelerated encoding
//! - Proper Retina/HiDPI handling
//! - Low memory footprint (streaming)
//! 
//! ## Example
//! ```no_run
//! use screen_capture::{Recorder, RecordingConfig};
//! 
//! let config = RecordingConfig {
//!     output_path: "recording.mp4".into(),
//!     fps: 30,
//!     capture_cursor: true,
//!     ..Default::default()
//! };
//! 
//! let mut recorder = Recorder::new(config)?;
//! recorder.start()?;
//! // ... record for some time ...
//! recorder.stop()?;
//! ```

use std::path::PathBuf;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use macos::NativeRecorder;
#[cfg(target_os = "windows")]
use windows::NativeRecorder;

/// Configuration for screen recording
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    /// Output MP4 file path
    pub output_path: PathBuf,
    
    /// Frames per second (default: 30)
    pub fps: u32,
    
    /// Video quality (0-100, default: 80)
    pub quality: u32,
    
    /// Capture mouse cursor (default: true)
    pub capture_cursor: bool,
    
    /// Display ID to capture (None = primary display)
    pub display_id: Option<u32>,
    
    /// Capture region (None = full screen)
    pub region: Option<CaptureRegion>,
    
    /// Capture microphone audio (default: false)
    pub capture_microphone: bool,
    
    /// Microphone device ID (None = default microphone)
    pub microphone_device_id: Option<String>,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("recording.mp4"),
            fps: 30,
            quality: 80,
            capture_cursor: true,
            display_id: None,
            region: None,
            capture_microphone: false,
            microphone_device_id: None,
        }
    }
}

/// Screen region to capture
#[derive(Debug, Clone, Copy)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Stopped,
}

/// Cross-platform screen recorder
pub struct Recorder {
    native: NativeRecorder,
    config: RecordingConfig,
    state: RecordingState,
}

impl Recorder {
    /// Create a new recorder with the given configuration
    pub fn new(config: RecordingConfig) -> Result<Self, String> {
        let native = NativeRecorder::new(&config)?;
        
        Ok(Self {
            native,
            config,
            state: RecordingState::Idle,
        })
    }
    
    /// Start recording
    pub fn start(&mut self) -> Result<(), String> {
        if self.state != RecordingState::Idle {
            return Err("Recorder is already running".to_string());
        }
        
        self.native.start()?;
        self.state = RecordingState::Recording;
        Ok(())
    }
    
    /// Stop recording and finalize the video file
    pub fn stop(&mut self) -> Result<PathBuf, String> {
        if self.state != RecordingState::Recording {
            return Err("Recorder is not recording".to_string());
        }
        
        self.native.stop()?;
        self.state = RecordingState::Stopped;
        Ok(self.config.output_path.clone())
    }
    
    /// Get current recording state
    pub fn state(&self) -> RecordingState {
        self.state
    }
    
    /// Get recording duration in seconds
    pub fn duration(&self) -> f64 {
        self.native.duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = RecordingConfig::default();
        assert_eq!(config.fps, 30);
        assert_eq!(config.quality, 80);
        assert!(config.capture_cursor);
    }
}
