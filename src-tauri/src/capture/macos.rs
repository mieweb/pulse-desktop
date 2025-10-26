// macOS-specific capture implementation using native ScreenCaptureKit

use std::path::PathBuf;
use std::time::Instant;
use screen_capture::{Recorder, RecordingConfig};
use log::{debug, info, error};

pub struct ScreenCapturer {
    output_folder: PathBuf,
    is_recording: bool,
    start_time: Option<Instant>,
    recorder: Option<Recorder>,
    mic_enabled: bool,
    pre_initialized: bool,
    prepared_output_path: Option<PathBuf>,
}

impl ScreenCapturer {
    pub fn new(output_folder: PathBuf, mic_enabled: bool) -> Self {
        Self {
            output_folder,
            is_recording: false,
            start_time: None,
            recorder: None,
            mic_enabled,
            pre_initialized: false,
            prepared_output_path: None,
        }
    }
    
    /// Pre-initialize the recorder (slow, ~2-3 seconds) so it's ready to start instantly
    pub async fn pre_initialize(&mut self, region: Option<(u32, u32, u32, u32)>) -> Result<(), String> {
        info!("🚀 Pre-initializing ScreenCaptureKit (this takes 2-3 seconds)...");
        let init_start = Instant::now();
        
        // Get next sequential recording path and store it
        let output_path = self.get_next_output_path();
        debug!("📝 Prepared output path: {:?}", output_path);
        
        // Convert region tuple to CaptureRegion if provided
        let capture_region = region.map(|(x, y, width, height)| {
            screen_capture::CaptureRegion { x, y, width, height }
        });
        
        // Create recording configuration
        let config = RecordingConfig {
            output_path: output_path.clone(),
            fps: 30,
            quality: 80,
            capture_cursor: true,
            capture_microphone: self.mic_enabled,
            microphone_device_id: None,
            display_id: Some(0),
            region: capture_region,
        };

        // Create recorder (this is the slow part - initializes ScreenCaptureKit)
        let recorder = Recorder::new(config)
            .map_err(|e| format!("Failed to create recorder: {}", e))?;

        self.recorder = Some(recorder);
        self.prepared_output_path = Some(output_path);
        self.pre_initialized = true;
        
        let init_duration = init_start.elapsed();
        info!("✅ ScreenCaptureKit pre-initialized in {:?}", init_duration);
        
        Ok(())
    }

    /// Request screen recording permission
    pub async fn request_permission() -> Result<bool, String> {
        // On macOS, ScreenCaptureKit will prompt for permission when needed
        debug!("📸 Screen recording permission will be requested on first capture");
        Ok(true)
    }

    /// Start recording the screen with optional region
    /// If pre_initialize() was called, this should be instant (<100ms)
    pub async fn start_recording(&mut self, _region: Option<(u32, u32, u32, u32)>) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".to_string());
        }

        let start_time = Instant::now();
        debug!("🎬 Starting native screen capture...");
        
        // Check if we have a pre-initialized recorder
        if !self.pre_initialized || self.recorder.is_none() {
            return Err("Recorder not pre-initialized! Call pre_initialize() first.".to_string());
        }
        
        // Use the prepared output path from pre-initialization
        if let Some(ref output_path) = self.prepared_output_path {
            debug!("💾 Output path: {:?}", output_path);
        } else {
            return Err("No prepared output path available".to_string());
        }

        // Start recording (should be instant if pre-initialized)
        if let Some(ref mut recorder) = self.recorder {
            recorder.start()
                .map_err(|e| format!("Failed to start recording: {}", e))?;

            let startup_duration = start_time.elapsed();
            info!("▶️  Recording started in {:?}", startup_duration);
            
            if startup_duration.as_millis() > 100 {
                error!("⚠️  SLOW START: {:?}", startup_duration);
                error!("💔 We sincerely apologize - you may have lost the first {:?} of your recording.", startup_duration);
            }
            
            self.is_recording = true;
            self.start_time = Some(Instant::now());
        } else {
            return Err("Recorder not initialized".to_string());
        }

        Ok(())
    }

    /// Stop recording and save the file
    pub async fn stop_recording(&mut self) -> Result<PathBuf, String> {
        if !self.is_recording {
            return Err("Not currently recording".to_string());
        }

        info!("⏹️  Stopping native screen capture...");
        
        let duration = self.start_time
            .map(|start| start.elapsed())
            .ok_or("No start time recorded")?;

        // Get the prepared output path before stopping
        let output_path = self.prepared_output_path.clone()
            .ok_or("No prepared output path available")?;

        // Stop the recorder
        if let Some(mut recorder) = self.recorder.take() {
            recorder.stop()
                .map_err(|e| format!("Failed to stop recording: {}", e))?;
            
            let recorded_duration = recorder.duration();
            
            info!("📊 Recording complete:");
            info!("  Wall clock duration: {:.2}s", duration.as_secs_f32());
            info!("  Recorded duration: {:.2}s", recorded_duration);
            info!("✅ Video saved to: {:?}", output_path);
            
            self.is_recording = false;
            self.pre_initialized = false; // Need to re-initialize for next recording
            self.prepared_output_path = None;
            
            Ok(output_path)
        } else {
            Err("No recorder available".to_string())
        }
    }

    /// Get the next sequential recording path (recording-1.mp4, recording-2.mp4, etc.)
    fn get_next_output_path(&self) -> PathBuf {
        let mut n = 1;
        loop {
            let path = self.output_folder.join(format!("recording-{}.mp4", n));
            if !path.exists() {
                return path;
            }
            n += 1;
        }
    }

    /// Get the most recently created recording path
    fn get_last_created_path(&self) -> PathBuf {
        // Find the highest numbered recording that exists
        let mut n = 1;
        let mut last_path = self.output_folder.join(format!("recording-{}.mp4", n));
        
        loop {
            let path = self.output_folder.join(format!("recording-{}.mp4", n));
            if !path.exists() {
                break;
            }
            last_path = path;
            n += 1;
        }
        
        last_path
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Get recording duration (if recording)
    pub fn duration(&self) -> f64 {
        if let Some(recorder) = &self.recorder {
            recorder.duration()
        } else {
            0.0
        }
    }
}
