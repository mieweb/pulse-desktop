# Native Screen Capture Crate - Summary

## What We Built

A new cross-platform Rust crate (`screen-capture`) that will replace our FFmpeg-based video encoding with native OS APIs.

## Structure

```
crates/screen-capture/
├── src/
│   ├── lib.rs          ✅ Public API (cross-platform)
│   ├── macos.rs        🚧 ScreenCaptureKit implementation (skeleton)
│   └── windows.rs      🚧 Desktop Duplication implementation (skeleton)
├── examples/
│   └── basic.rs        ✅ Example usage
├── Cargo.toml          ✅ Dependencies configured
├── README.md           ✅ Documentation
└── IMPLEMENTATION_PLAN.md  ✅ Development roadmap
```

## Current Status

✅ **Complete**:
- Project structure
- Public API design
- Platform detection (macOS/Windows)
- Example code
- Documentation
- Compiles successfully

🚧 **In Progress**:
- macOS ScreenCaptureKit implementation (skeleton only)
- Windows Desktop Duplication implementation (skeleton only)

❌ **Not Started**:
- Objective-C/Swift bridge for macOS
- COM/Windows API integration
- Integration with pulse-desktop
- Testing

## API Example

```rust
use screen_capture::{Recorder, RecordingConfig};

// Create recorder
let config = RecordingConfig {
    output_path: "recording.mp4".into(),
    fps: 30,
    quality: 80,
    capture_cursor: true,
    ..Default::default()
};

let mut recorder = Recorder::new(config)?;

// Record
recorder.start()?;
std::thread::sleep(Duration::from_secs(5));
recorder.stop()?;
```

## Why This Approach?

### Current (FFmpeg) Problems:
1. **Retina scaling bug**: Captured 2940×1912 but encoder expected 1470×956
2. **High memory**: Buffering all frames as PNGs
3. **CPU intensive**: PNG decode → RGB → YUV → H.264 transcode
4. **Large files**: Software encoder not as efficient
5. **Complex dependencies**: FFmpeg system requirement

### Native API Benefits:
1. **Perfect Retina**: Native APIs handle scaling correctly
2. **Low memory**: Streaming directly to file
3. **Fast**: Hardware encoding (VideoToolbox/Media Foundation)
4. **Small files**: Better compression from hardware encoder
5. **No dependencies**: Uses OS built-in frameworks

## Platform APIs

### macOS (ScreenCaptureKit)
```
SCShareableContent → SCStream → CMSampleBuffer → AVAssetWriter → MP4
                     ↓
              VideoToolbox (H.264 hardware encoding)
```

### Windows (Desktop Duplication)
```
DXGI Duplication → ID3D11Texture2D → IMFSample → IMFSinkWriter → MP4
                   ↓
           Media Foundation (H.264 hardware encoding)
```

## Next Steps

1. **Implement macOS first**:
   - Create Objective-C bridge
   - Integrate ScreenCaptureKit
   - Test Retina capture
   
2. **Test in pulse-desktop**:
   - Replace `screenshots` crate
   - Update capture module
   - Verify push-to-hold works
   
3. **Implement Windows**:
   - COM initialization
   - Desktop Duplication API
   - Media Foundation encoder

## Timeline

- **Week 1**: macOS implementation + integration
- **Week 2**: Windows implementation
- **Week 3**: Testing + polish
- **Week 4**: Publish to crates.io

## Decision

This is the **correct long-term architecture**. The FFmpeg approach was a quick proof-of-concept, but native APIs are the proper solution for production.

---

**Created**: October 12, 2025  
**Status**: Foundation complete, implementation pending  
**Blocked by**: Need to implement Objective-C bridge for ScreenCaptureKit
