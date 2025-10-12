# Native Screen Capture Implementation Plan

## Overview

Replacing the current FFmpeg-based approach with native screen capture APIs for direct MP4 encoding.

## Architecture

```
screen-capture (crate)
├── Public API (platform-agnostic)
│   ├── Recorder
│   ├── RecordingConfig
│   └── CaptureRegion
├── macOS (ScreenCaptureKit + AVAssetWriter)
│   ├── SCStream for frame capture
│   ├── AVAssetWriter for MP4 muxing
│   └── VideoToolbox for H.264 encoding
└── Windows (Desktop Duplication + Media Foundation)
    ├── IDXGIOutputDuplication for frame capture
    ├── IMFSinkWriter for MP4 muxing
    └── Media Foundation for H.264 encoding
```

## Implementation Phases

### Phase 1: macOS ScreenCaptureKit ✅ (Started)
- [x] Create crate structure
- [x] Define public API
- [ ] Implement Objective-C bridge
  - [ ] SCShareableContent (list displays)
  - [ ] SCStreamConfiguration (fps, size, format)
  - [ ] SCStream + delegate (receive frames)
  - [ ] AVAssetWriter (MP4 writer)
  - [ ] AVAssetWriterInput (video track)
- [ ] Handle CMSampleBuffer → AVAssetWriter pipeline
- [ ] Test with Retina displays
- [ ] Permission handling

### Phase 2: Windows Desktop Duplication
- [ ] COM initialization
- [ ] D3D11 device setup
- [ ] DXGI output duplication
- [ ] Media Foundation sink writer
- [ ] H.264 encoder configuration
- [ ] Frame capture loop
- [ ] Test with multiple monitors

### Phase 3: Integration
- [ ] Replace screenshots crate in pulse-desktop
- [ ] Update macos.rs to use screen-capture crate
- [ ] Remove FFmpeg dependencies
- [ ] Update encoding module (delete?)
- [ ] Test push-to-hold workflow
- [ ] Verify sequential file naming

### Phase 4: Polish
- [ ] Error handling improvements
- [ ] Audio capture (optional)
- [ ] Pause/resume support
- [ ] Region selection integration
- [ ] Performance benchmarks
- [ ] Documentation
- [ ] Publish crate to crates.io

## API Design (Final)

```rust
use screen_capture::{Recorder, RecordingConfig};

// Simple usage
let config = RecordingConfig::default()
    .output_path("recording.mp4")
    .fps(30)
    .quality(80);

let mut recorder = Recorder::new(config)?;
recorder.start()?;
// ... record ...
recorder.stop()?;
```

## Benefits vs Current Approach

| Feature | Current (FFmpeg) | Native APIs |
|---------|-----------------|-------------|
| Memory | High (frame buffer) | Low (streaming) |
| CPU | High (transcode) | Low (hw encode) |
| Retina | Buggy (scaling issues) | Perfect (native) |
| File size | Larger | Smaller (hw encoder) |
| Quality | Good | Excellent |
| Latency | High | Low |
| Dependencies | FFmpeg required | OS built-in |

## Technical Details

### macOS ScreenCaptureKit

```swift
// Conceptual flow
SCShareableContent.getWithCompletionHandler { content in
    let display = content.displays.first
    let config = SCStreamConfiguration()
    config.width = display.width
    config.height = display.height
    config.minimumFrameInterval = CMTime(1, 30) // 30 fps
    
    let stream = SCStream(filter: filter, configuration: config, delegate: self)
    stream.startCapture()
}

// Delegate receives CMSampleBuffer
func stream(_ stream: SCStream, didOutput sample: CMSampleBuffer) {
    assetWriter.append(sample)
}
```

### Windows Desktop Duplication

```cpp
// Conceptual flow
D3D11CreateDevice(&device);
output->DuplicateOutput(device, &duplication);

// Capture loop
duplication->AcquireNextFrame(&frameInfo, &desktopResource);
// Convert to Media Foundation sample
sinkWriter->WriteSample(videoStreamIndex, sample);
duplication->ReleaseFrame();
```

## Testing Strategy

1. **Basic capture**: 5s recording, verify MP4 playable
2. **Retina displays**: Check resolution matches physical pixels
3. **Multiple monitors**: Capture secondary display
4. **Region selection**: Capture 1920×1080 region
5. **Long recordings**: 5 minutes, check memory usage
6. **Rapid start/stop**: Verify file integrity
7. **Permission denial**: Graceful error handling

## Migration Path

1. Implement macOS first (working prototype)
2. Test in pulse-desktop alongside existing code
3. Feature flag to switch between old/new
4. Once stable, remove FFmpeg approach
5. Implement Windows version
6. Remove feature flag, native-only

## Timeline Estimate

- Phase 1 (macOS): 2-3 days
- Phase 2 (Windows): 2-3 days  
- Phase 3 (Integration): 1 day
- Phase 4 (Polish): 1-2 days

**Total**: ~1 week for full implementation

## Next Steps

1. ✅ Create crate structure
2. ✅ Define API
3. ⏳ **Next**: Implement Objective-C bridge for ScreenCaptureKit
4. Test basic capture on macOS
5. Integrate into pulse-desktop

---

**Status**: 🚧 Phase 1 in progress (API defined, implementation pending)
