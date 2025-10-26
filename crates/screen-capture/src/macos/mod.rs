// macOS implementation using ScreenCaptureKit + AVAssetWriter
//
// This uses modern macOS APIs (10.15+) for efficient screen capture
// directly to MP4 with hardware encoding.

mod bridge;
use bridge::ScreenCaptureRecorder;
use crate::{RecordingConfig, CaptureRegion};
use log::{info, debug, warn, error};
use std::ffi::CStr;
use std::os::raw::c_char;

/// C-callable logging function for Objective-C code
/// This allows Objective-C to log through Rust's log system with delta timestamps
#[no_mangle]
pub extern "C" fn rust_log_info(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    unsafe {
        if let Ok(s) = CStr::from_ptr(msg).to_str() {
            info!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_log_debug(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    unsafe {
        if let Ok(s) = CStr::from_ptr(msg).to_str() {
            debug!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_log_warn(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    unsafe {
        if let Ok(s) = CStr::from_ptr(msg).to_str() {
            warn!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_log_error(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    unsafe {
        if let Ok(s) = CStr::from_ptr(msg).to_str() {
            error!("{}", s);
        }
    }
}

pub struct NativeRecorder {
    recorder: Option<ScreenCaptureRecorder>,
    config: RecordingConfig,
}

impl NativeRecorder {
    pub fn new(config: &RecordingConfig) -> Result<Self, String> {
        info!("🚀 Initializing macOS ScreenCaptureKit recorder");
        
        // Get display dimensions
        // For now, use primary display dimensions (0 = main display)
        // TODO: Get actual display dimensions from Core Graphics
        let display_id = config.display_id.unwrap_or(0);
        
        // For Retina displays, we'll capture at physical resolution
        // ScreenCaptureKit handles this automatically
        let (width, height) = if let Some(region) = config.region {
            (region.width, region.height)
        } else {
            // TODO: Query actual display size
            // For now, use a reasonable default
            (1920, 1080)
        };
        
        let recorder = ScreenCaptureRecorder::new(
            config.output_path.to_str().unwrap(),
            width,
            height,
            config.fps,
            config.quality,
            display_id,
            config.capture_microphone,
        )?;
        
        Ok(Self {
            recorder: Some(recorder),
            config: config.clone(),
        })
    }
    
    pub fn start(&mut self) -> Result<(), String> {
        if let Some(recorder) = &mut self.recorder {
            recorder.start()?;
            info!("▶️  ScreenCaptureKit recording started");
            Ok(())
        } else {
            Err("Recorder not initialized".to_string())
        }
    }
    
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(recorder) = &mut self.recorder {
            recorder.stop()?;
            info!("⏹️  ScreenCaptureKit recording stopped");
            Ok(())
        } else {
            Err("Recorder not initialized".to_string())
        }
    }
    
    pub fn duration(&self) -> f64 {
        self.recorder
            .as_ref()
            .map(|r| r.duration())
            .unwrap_or(0.0)
    }
}
