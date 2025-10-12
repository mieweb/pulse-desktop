# Screen Capture Implementation Plan

## Research: Available Rust Crates for macOS Screen Recording

### Option 1: screencapturekit-rs (Recommended)
- **Crate**: `screencapturekit` and `screencapturekit-sys`
- **Pros**: 
  - Native ScreenCaptureKit bindings
  - Modern Apple API (macOS 12.3+)
  - Best performance and quality
  - Hardware-accelerated encoding
- **Cons**: 
  - Requires macOS 12.3 or later
  - Complex API to learn
  - May require Objective-C bridging

### Option 2: scap (Screen Capture Library)
- **Crate**: `scap`
- **Pros**:
  - Cross-platform (macOS, Windows, Linux)
  - Simple Rust API
  - Good for frame capture
- **Cons**:
  - Less control than native APIs
  - May not support push-to-hold workflow well
  - Performance concerns for video recording

### Option 3: AVFoundation via objc2
- **Crate**: `objc2`, `objc2-foundation`, `objc2-av-foundation`
- **Pros**:
  - Full control over capture pipeline
  - Mature and well-tested
  - Works on older macOS versions
- **Cons**:
  - More boilerplate code
  - Need to write Objective-C FFI
  - Deprecated API (ScreenCaptureKit is newer)

### Option 4: ffmpeg-sys-next (Video Processing)
- **Crate**: `ffmpeg-sys-next` or `ffmpeg-next`
- **Pros**:
  - Powerful encoding capabilities
  - Cross-platform
  - Flexible format support
- **Cons**:
  - Requires FFmpeg system dependency
  - Complex API
  - Overkill for simple recording

## Decision: Hybrid Approach

For V0, let's use a **pragmatic hybrid approach**:

1. **Frame Capture**: Use `scap` for simple cross-platform frame capture
2. **Video Encoding**: Use `mp4` crate for MP4 container + H.264 encoding
3. **Future**: Migrate to ScreenCaptureKit when we need advanced features

### Why This Approach?
- ✅ **Fast to implement**: scap has a simple API
- ✅ **Cross-platform**: Works on macOS and Windows
- ✅ **Good enough for V0**: Full screen capture works
- ✅ **Proven**: Used by similar projects
- ⚠️ **Migration path**: Can switch to ScreenCaptureKit later for better quality

## Implementation Steps

### Phase 1: Basic Frame Capture (This Session)
1. Add dependencies to Cargo.toml
2. Implement frame capture with scap
3. Store frames in memory buffer
4. Wire up to hotkey press/release

### Phase 2: Video Encoding (Next Session)
1. Add mp4 encoding dependencies
2. Encode frames to H.264
3. Write to MP4 file
4. Handle timing and framerate

### Phase 3: File Management (Following Session)
1. Sequential file naming
2. Duration tracking
3. Cleanup and error handling

## Dependencies to Add

```toml
[dependencies]
# Screen capture
scap = "0.1.4"

# Video encoding
mp4 = "0.14"
# OR ffmpeg-next = "7.0" (if we need more control)

# Image processing
image = "0.25"

# Async runtime (already have via Tauri)
tokio = { version = "1", features = ["full"] }
```

## Minimal Viable Implementation

### Goal
Capture frames while hotkey is held, save as MP4 when released.

### Pseudo-code
```rust
struct Recorder {
    capturer: scap::Capturer,
    frames: Vec<Frame>,
    start_time: Instant,
}

impl Recorder {
    async fn start() {
        // Start capturing frames at ~30 fps
        loop {
            let frame = capturer.capture_frame();
            frames.push(frame);
            sleep(33ms); // ~30 fps
        }
    }

    async fn stop() -> PathBuf {
        // Encode frames to MP4
        let encoder = Mp4Encoder::new();
        for frame in frames {
            encoder.add_frame(frame);
        }
        encoder.save(path)
    }
}
```

## Acceptance Criteria

- [ ] Frame capture works at 30 fps
- [ ] Recording starts on hotkey press
- [ ] Recording stops on hotkey release
- [ ] MP4 file is created and playable
- [ ] Duration matches hold time (±150ms)
- [ ] No memory leaks after multiple recordings

## Known Limitations (V0)

- No region selection (full screen only)
- No scaling (captures at native resolution)
- No audio (will add in Priority 3)
- No system audio (V0 scope)
- ~30 fps only (no 60fps yet)
- macOS 10.15+ required

## Next Steps After Implementation

1. Test on different screen resolutions
2. Add error handling for capture failures
3. Implement file management (Priority 2)
4. Add microphone audio (Priority 3)
5. Migrate to ScreenCaptureKit for better quality (Future)
