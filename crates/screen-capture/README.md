# screen-capture

Cross-platform native video screen capture for Rust.

## Features

- ✅ **Native APIs**: Uses platform-specific APIs for maximum performance
  - **macOS**: ScreenCaptureKit + AVAssetWriter
  - **Windows**: Desktop Duplication API + Media Foundation
- ✅ **Direct MP4 encoding**: No transcoding overhead
- ✅ **Hardware acceleration**: Uses VideoToolbox (macOS) and Media Foundation (Windows)
- ✅ **Retina/HiDPI support**: Proper scaling on high-resolution displays
- ✅ **Low memory**: Streaming architecture, not frame buffering
- ✅ **Clean API**: Simple, ergonomic interface

## Usage

```rust
use screen_capture::{Recorder, RecordingConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    
    // Record for 5 seconds
    std::thread::sleep(std::time::Duration::from_secs(5));
    
    // Stop and save
    let output_path = recorder.stop()?;
    println!("Recording saved to: {:?}", output_path);
    
    Ok(())
}
```

## Region Capture

```rust
use screen_capture::{Recorder, RecordingConfig, CaptureRegion};

let config = RecordingConfig {
    output_path: "region.mp4".into(),
    region: Some(CaptureRegion {
        x: 100,
        y: 100,
        width: 1920,
        height: 1080,
    }),
    ..Default::default()
};

let mut recorder = Recorder::new(config)?;
recorder.start()?;
```

## Platform Requirements

### macOS
- macOS 10.15+ (for ScreenCaptureKit)
- Screen Recording permission granted

### Windows
- Windows 10+ (for Desktop Duplication API)
- DirectX 11 compatible GPU

## Implementation Status

- [x] API design
- [x] Project structure
- [ ] macOS ScreenCaptureKit implementation
- [ ] Windows Desktop Duplication implementation
- [ ] Audio capture support
- [ ] Pause/resume functionality
- [ ] Multiple monitor selection

## Architecture

```
screen-capture/
├── src/
│   ├── lib.rs           # Public API, cross-platform types
│   ├── macos.rs         # ScreenCaptureKit + AVAssetWriter
│   └── windows.rs       # Desktop Duplication + Media Foundation
├── examples/
│   ├── basic.rs         # Simple recording example
│   └── region.rs        # Region capture example
└── Cargo.toml
```

## License

MIT
