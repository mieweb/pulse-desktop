// macOS-specific capture implementation using native ScreenCaptureKit

use std::path::PathBuf;
use std::time::Instant;
use screen_capture::{Recorder, RecordingConfig};

pub struct ScreenCapturer {
    output_path: PathBuf,
    is_recording: bool,
    start_time: Option<Instant>,
    recorder: Option<Recorder>,
    mic_enabled: bool,
}

impl ScreenCapturer {
    pub fn new(output_path: PathBuf, mic_enabled: bool) -> Self {
        Self {
            output_path,
            is_recording: false,
            start_time: None,
            recorder: None,
            mic_enabled,
        }
    }

    /// Request screen recording permission
    pub async fn request_permission() -> Result<bool, String> {
        // On macOS, ScreenCaptureKit will prompt for permission when needed
        println!("📸 Screen recording permission will be requested on first capture");
        Ok(true)
    }

    /// Start recording the screen with optional region
    pub async fn start_recording(&mut self, region: Option<(u32, u32, u32, u32)>) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".to_string());
        }

        println!("🎬 Starting native screen capture...");
        
        // Get next sequential recording path
        let output_path = self.get_next_output_path();
        println!("💾 Output path: {:?}", output_path);
        
        // Convert region tuple to CaptureRegion if provided
        let capture_region = region.map(|(x, y, width, height)| {
            screen_capture::CaptureRegion { x, y, width, height }
        });
        
        if let Some(ref region) = capture_region {
            println!("📏 Using capture region: {}×{} at ({}, {})", 
                     region.width, region.height, region.x, region.y);
        } else {
            println!("🖥️ Using full screen capture");
        }
        
        // Create recording configuration
        let config = RecordingConfig {
            output_path: output_path.clone(),
            fps: 30,
            quality: 80,
            capture_cursor: true,
            capture_microphone: self.mic_enabled,  // Use setting from AppState
            microphone_device_id: None,  // Use default microphone
            display_id: Some(0), // Primary display
            region: capture_region,  // Use provided region or None for full screen
        };

        // Create recorder
        let mut recorder = Recorder::new(config)
            .map_err(|e| format!("Failed to create recorder: {}", e))?;

        // Start recording
        recorder.start()
            .map_err(|e| format!("Failed to start recording: {}", e))?;

        println!("▶️  Recording started");
        
        self.recorder = Some(recorder);
        self.is_recording = true;
        self.start_time = Some(Instant::now());

        Ok(())
    }

    /// Stop recording and save the file
    pub async fn stop_recording(&mut self) -> Result<PathBuf, String> {
        if !self.is_recording {
            return Err("Not currently recording".to_string());
        }

        println!("⏹️  Stopping native screen capture...");
        
        let duration = self.start_time
            .map(|start| start.elapsed())
            .ok_or("No start time recorded")?;

        // Stop the recorder
        if let Some(mut recorder) = self.recorder.take() {
            recorder.stop()
                .map_err(|e| format!("Failed to stop recording: {}", e))?;
            
            let recorded_duration = recorder.duration();
            
            println!("📊 Recording complete:");
            println!("  Wall clock duration: {:.2}s", duration.as_secs_f32());
            println!("  Recorded duration: {:.2}s", recorded_duration);
            
            // Get the output path from the recorder's config
            let output_path = self.get_last_created_path();
            println!("✅ Video saved to: {:?}", output_path);
            
            self.is_recording = false;
            
            Ok(output_path)
        } else {
            Err("No recorder available".to_string())
        }
    }

    /// Get the next sequential recording path (1.mp4, 2.mp4, etc.)
    fn get_next_output_path(&self) -> PathBuf {
        let mut n = 1;
        loop {
            let path = self.output_path.join(format!("{}.mp4", n));
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
        let mut last_path = self.output_path.join(format!("{}.mp4", n));
        
        loop {
            let path = self.output_path.join(format!("{}.mp4", n));
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
