// Test the screen-capture crate on macOS
use screen_capture::{Recorder, RecordingConfig};
use std::time::Duration;
use std::thread;

fn main() -> Result<(), String> {
    println!("🎬 Testing native ScreenCaptureKit recorder...\n");
    
    let config = RecordingConfig {
        output_path: "/tmp/test-recording.mp4".into(),
        fps: 30,
        quality: 80,
        capture_cursor: true,
        ..Default::default()
    };

    println!("📝 Config:");
    println!("   Output: {:?}", config.output_path);
    println!("   FPS: {}", config.fps);
    println!("   Quality: {}\n", config.quality);

    let mut recorder = Recorder::new(config)?;
    println!("✅ Recorder created");
    
    // Start recording
    recorder.start()?;
    println!("▶️  Recording started...\n");
    
    // Record for 3 seconds
    for i in 1..=3 {
        thread::sleep(Duration::from_secs(1));
        println!("   {} second(s) - Duration: {:.2}s", i, recorder.duration());
    }
    
    // Stop and save
    println!("\n⏹️  Stopping...");
    let output_path = recorder.stop()?;
    println!("✅ Recording saved to: {:?}\n", output_path);
    
    // Verify file exists
    if output_path.exists() {
        let metadata = std::fs::metadata(&output_path).unwrap();
        println!("📊 File size: {} bytes", metadata.len());
        println!("🎉 SUCCESS: Native ScreenCaptureKit recording works!");
    } else {
        return Err("File not found after recording".to_string());
    }
    
    Ok(())
}
