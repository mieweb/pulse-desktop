// macOS-specific capture implementation using screenshots library

use std::path::PathBuf;
use std::time::Instant;
use screenshots::Screen;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::encoding::VideoEncoder;

pub struct ScreenCapturer {
    output_path: PathBuf,
    is_recording: bool,
    frames: Arc<Mutex<Vec<Vec<u8>>>>,
    frame_info: Arc<Mutex<Option<(u32, u32)>>>, // width, height
    start_time: Option<Instant>,
    capture_thread: Option<thread::JoinHandle<()>>,
    stop_signal: Arc<Mutex<bool>>,
}

impl ScreenCapturer {
    pub fn new(output_path: PathBuf) -> Self {
        Self {
            output_path,
            is_recording: false,
            frames: Arc::new(Mutex::new(Vec::new())),
            frame_info: Arc::new(Mutex::new(None)),
            start_time: None,
            capture_thread: None,
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }

    /// Request screen recording permission
    pub async fn request_permission() -> Result<bool, String> {
        // On macOS, screenshots will prompt for permission when needed
        println!("📸 Screen capture permission will be requested on first capture");
        Ok(true)
    }

    /// Start recording the screen
    pub async fn start_recording(&mut self) -> Result<(), String> {
        if self.is_recording {
            return Err("Already recording".to_string());
        }

        println!("�� Starting screen capture...");
        
        // Get primary screen
        let screens = Screen::all()
            .map_err(|e| format!("Failed to get screens: {}", e))?;
        
        if screens.is_empty() {
            return Err("No screens found".to_string());
        }

        let screen = screens[0].clone();
        println!("📺 Capturing display: {} ({}x{})", 
            screen.display_info.id, 
            screen.display_info.width, 
            screen.display_info.height
        );

        // Store frame dimensions
        {
            let mut info = self.frame_info.lock().unwrap();
            *info = Some((screen.display_info.width, screen.display_info.height));
        }

        self.is_recording = true;
        self.start_time = Some(Instant::now());
        
        // Clear previous state
        {
            let mut frames = self.frames.lock().unwrap();
            frames.clear();
            let mut stop = self.stop_signal.lock().unwrap();
            *stop = false;
        }

        // Spawn capture thread
        let frames_clone = Arc::clone(&self.frames);
        let stop_clone = Arc::clone(&self.stop_signal);
        
        let handle = thread::spawn(move || {
            let target_fps = 30;
            let frame_duration = std::time::Duration::from_millis(1000 / target_fps);
            let mut frame_count = 0;
            
            loop {
                let loop_start = Instant::now();
                
                // Check stop signal
                {
                    let stop = stop_clone.lock().unwrap();
                    if *stop {
                        println!("🛑 Capture thread received stop signal");
                        break;
                    }
                }
                
                // Capture frame
                match screen.capture() {
                    Ok(image) => {
                        // Get actual image dimensions (may be scaled for Retina)
                        let actual_width = image.width();
                        let actual_height = image.height();
                        
                        // Update frame info with actual dimensions on first frame
                        if frame_count == 0 {
                            let mut info = self.frame_info.lock().unwrap();
                            *info = Some((actual_width, actual_height));
                            println!("📐 Actual capture resolution: {}x{}", actual_width, actual_height);
                        }
                        
                        let buffer = image.to_png(None)
                            .expect("Failed to convert to PNG");
                        
                        let mut frames = frames_clone.lock().unwrap();
                        frames.push(buffer);
                        frame_count += 1;
                        
                        if frame_count % 30 == 0 {
                            println!("📸 Captured {} frames ({}x{})", 
                                frame_count,
                                screen.display_info.width,
                                screen.display_info.height
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Error capturing frame: {}", e);
                        break;
                    }
                }
                
                // Maintain target FPS
                let elapsed = loop_start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
            
            println!("✅ Capture thread finished with {} frames", frame_count);
        });

        self.capture_thread = Some(handle);
        
        Ok(())
    }

    /// Stop recording and save the file
    pub async fn stop_recording(&mut self) -> Result<PathBuf, String> {
        if !self.is_recording {
            return Err("Not currently recording".to_string());
        }

        println!("⏹️  Stopping screen capture...");
        
        // Signal capture thread to stop
        {
            let mut stop = self.stop_signal.lock().unwrap();
            *stop = true;
        }
        
        self.is_recording = false;
        
        // Wait for capture thread to finish
        if let Some(handle) = self.capture_thread.take() {
            if let Err(e) = handle.join() {
                eprintln!("❌ Capture thread panicked: {:?}", e);
            }
        }

        let duration = self.start_time
            .map(|start| start.elapsed())
            .ok_or("No start time recorded")?;

        let frames = self.frames.lock().unwrap().clone();
        let frame_count = frames.len();
        
        if frame_count == 0 {
            return Err("No frames captured".to_string());
        }
        
        // Get frame dimensions
        let (width, height) = self.frame_info.lock().unwrap()
            .ok_or("No frame info available")?;
        
        println!("📊 Recording complete:");
        println!("  Duration: {:.2}s", duration.as_secs_f32());
        println!("  Frames: {}", frame_count);
        println!("  Resolution: {}×{}", width, height);
        println!("  Average FPS: {:.2}", frame_count as f32 / duration.as_secs_f32());

        // Get next sequential recording number
        let output_path = self.get_next_output_path();
        println!("💾 Output path: {:?}", output_path);

        // Create video encoder
        let encoder = VideoEncoder::new(width, height, 30);
        
        // Calculate expected duration
        let expected_duration = encoder.calculate_duration(frame_count);
        println!("📺 Expected video duration: {:.2}s", expected_duration);

        // Encode frames to MP4
        encoder.encode_to_mp4(frames, output_path.clone())?;
        
        println!("✅ Video saved successfully!");

        Ok(output_path)
    }

    /// Get the next sequential recording path (recording-1.mp4, recording-2.mp4, etc.)
    fn get_next_output_path(&self) -> PathBuf {
        let mut n = 1;
        loop {
            let path = self.output_path.join(format!("recording-{}.mp4", n));
            if !path.exists() {
                return path;
            }
            n += 1;
        }
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.lock().unwrap().len()
    }
}
