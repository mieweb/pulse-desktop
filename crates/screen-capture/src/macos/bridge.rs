// Objective-C/Swift bridge for ScreenCaptureKit
//
// This file provides Rust FFI bindings to macOS ScreenCaptureKit API

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::ptr;

// Opaque types for Objective-C objects
#[repr(C)]
pub struct SCRecorder {
    _private: [u8; 0],
}

// Callback type for recording events
pub type SCRecorderCallback = extern "C" fn(event: i32, user_data: *mut c_void);

// Events
pub const SC_EVENT_STARTED: i32 = 0;
pub const SC_EVENT_STOPPED: i32 = 1;
pub const SC_EVENT_ERROR: i32 = 2;
pub const SC_EVENT_FRAME: i32 = 3;

extern "C" {
    // Create a new recorder
    // Returns NULL on failure
    pub fn sc_recorder_create(
        output_path: *const c_char,
        width: u32,
        height: u32,
        fps: u32,
        quality: u32,
        display_id: u32,
    ) -> *mut SCRecorder;
    
    // Start recording
    // Returns 0 on success, non-zero on error
    pub fn sc_recorder_start(recorder: *mut SCRecorder) -> i32;
    
    // Stop recording
    // Returns 0 on success, non-zero on error
    pub fn sc_recorder_stop(recorder: *mut SCRecorder) -> i32;
    
    // Get recording duration in seconds
    pub fn sc_recorder_duration(recorder: *mut SCRecorder) -> f64;
    
    // Free the recorder
    pub fn sc_recorder_free(recorder: *mut SCRecorder);
    
    // Set callback for events
    pub fn sc_recorder_set_callback(
        recorder: *mut SCRecorder,
        callback: SCRecorderCallback,
        user_data: *mut c_void,
    );
    
    // Get last error message (NULL if no error)
    pub fn sc_recorder_last_error(recorder: *mut SCRecorder) -> *const c_char;
}

// Safe Rust wrapper
pub struct ScreenCaptureRecorder {
    recorder: *mut SCRecorder,
}

unsafe impl Send for ScreenCaptureRecorder {}
unsafe impl Sync for ScreenCaptureRecorder {}

impl ScreenCaptureRecorder {
    pub fn new(
        output_path: &str,
        width: u32,
        height: u32,
        fps: u32,
        quality: u32,
        display_id: u32,
    ) -> Result<Self, String> {
        let path_cstr = CString::new(output_path)
            .map_err(|e| format!("Invalid path: {}", e))?;
        
        let recorder = unsafe {
            sc_recorder_create(
                path_cstr.as_ptr(),
                width,
                height,
                fps,
                quality,
                display_id,
            )
        };
        
        if recorder.is_null() {
            return Err("Failed to create recorder".to_string());
        }
        
        Ok(Self { recorder })
    }
    
    pub fn start(&mut self) -> Result<(), String> {
        let result = unsafe { sc_recorder_start(self.recorder) };
        if result != 0 {
            let error = self.get_error();
            return Err(format!("Failed to start: {}", error));
        }
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<(), String> {
        let result = unsafe { sc_recorder_stop(self.recorder) };
        if result != 0 {
            let error = self.get_error();
            return Err(format!("Failed to stop: {}", error));
        }
        Ok(())
    }
    
    pub fn duration(&self) -> f64 {
        unsafe { sc_recorder_duration(self.recorder) }
    }
    
    fn get_error(&self) -> String {
        unsafe {
            let err_ptr = sc_recorder_last_error(self.recorder);
            if err_ptr.is_null() {
                return "Unknown error".to_string();
            }
            std::ffi::CStr::from_ptr(err_ptr)
                .to_string_lossy()
                .to_string()
        }
    }
}

impl Drop for ScreenCaptureRecorder {
    fn drop(&mut self) {
        if !self.recorder.is_null() {
            unsafe { sc_recorder_free(self.recorder) };
        }
    }
}
