// Basic screen recording example
//
// Records 5 seconds of screen to recording.mp4

use screen_capture::{Recorder, RecordingConfig};
use std::time::Duration;

fn main() -> Result<(), String> {
    println!("🎬 Starting screen recording...");
    
    let config = RecordingConfig {
        output_path: "recording.mp4".into(),
        fps: 30,
        quality: 80,
        capture_cursor: true,
        ..Default::default()
    };

    let mut recorder = Recorder::new(config)?;
    
    // Start recording
    recorder.start()?;
    println!("▶️  Recording... (5 seconds)");
    
    // Record for 5 seconds
    std::thread::sleep(Duration::from_secs(5));
    
    // Stop and save
    let output_path = recorder.stop()?;
    println!("✅ Recording saved to: {:?}", output_path);
    
    Ok(())
}
