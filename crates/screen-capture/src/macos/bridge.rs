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
    // audio_device_id: optional device unique ID (NULL for auto-select)
    pub fn sc_recorder_create(
        output_path: *const c_char,
        width: u32,
        height: u32,
        fps: u32,
        quality: u32,
        display_id: u32,
        capture_audio: bool,
        audio_device_id: *const c_char,
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
    
    // Audio device management
    pub fn sc_get_audio_devices() -> *mut AudioDeviceList;
    pub fn sc_free_audio_device_list(list: *mut AudioDeviceList);
}

// Audio device structures
#[repr(C)]
pub struct AudioDeviceInfo {
    pub device_id: *mut c_char,
    pub device_name: *mut c_char,
    pub is_default: bool,
    pub is_builtin: bool,
}

#[repr(C)]
pub struct AudioDeviceList {
    pub devices: *mut AudioDeviceInfo,
    pub count: usize,
}

// Rust-safe audio device representation
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub is_builtin: bool,
}

pub fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    unsafe {
        let list_ptr = sc_get_audio_devices();
        if list_ptr.is_null() {
            return Err("Failed to get audio devices".to_string());
        }
        
        let list = &*list_ptr;
        let mut devices = Vec::with_capacity(list.count);
        
        for i in 0..list.count {
            let device = &*list.devices.add(i);
            
            let id = if device.device_id.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(device.device_id)
                    .to_string_lossy()
                    .to_string()
            };
            
            let name = if device.device_name.is_null() {
                String::new()
            } else {
                std::ffi::CStr::from_ptr(device.device_name)
                    .to_string_lossy()
                    .to_string()
            };
            
            devices.push(AudioDevice {
                id,
                name,
                is_default: device.is_default,
                is_builtin: device.is_builtin,
            });
        }
        
        sc_free_audio_device_list(list_ptr);
        Ok(devices)
    }
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
        capture_audio: bool,
        audio_device_id: Option<&str>,
    ) -> Result<Self, String> {
        let path_cstr = CString::new(output_path)
            .map_err(|e| format!("Invalid path: {}", e))?;
        
        // Convert optional device ID to C string
        let device_id_cstr = audio_device_id
            .map(|id| CString::new(id).ok())
            .flatten();
        let device_id_ptr = device_id_cstr
            .as_ref()
            .map(|cs| cs.as_ptr())
            .unwrap_or(ptr::null());
        
        let recorder = unsafe {
            sc_recorder_create(
                path_cstr.as_ptr(),
                width,
                height,
                fps,
                quality,
                display_id,
                capture_audio,
                device_id_ptr,
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
