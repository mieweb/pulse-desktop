# Audio Capture Implementation Plan

**Status**: 🟡 TODO  
**Priority**: Medium (Core feature for V0)

## Current State

✅ **Video capture working perfectly**
- ScreenCaptureKit for video
- H.264 hardware encoding
- Direct MP4 output

❌ **No audio capture yet**
- Current implementation: video-only
- Microphone capture needed for V0
- System audio NOT in V0 scope

## Requirements (from copilot-instructions.md)

### V0 Scope
- ✅ **Video**: Screen capture with ScreenCaptureKit
- 🟡 **Microphone audio**: Toggle mic recording on/off
- ❌ **System audio**: Deferred to future version

### Testing Requirements
- [ ] Mic toggle: Audio present/absent based on toggle state
- [ ] Recording accuracy: Hold duration matches file duration (±150ms)

## Implementation Plan

### Phase 1: Add Audio Configuration (30 min)
**Goal**: Extend `RecordingConfig` to support audio settings

1. Add to `RecordingConfig`:
   ```rust
   pub capture_microphone: bool,  // Default: false
   pub microphone_device_id: Option<String>,  // None = default mic
   ```

2. Update `Default` implementation

### Phase 2: macOS Audio Capture (2-3 hours)
**Goal**: Capture microphone using AVCaptureDevice

#### Objective-C Implementation
1. Add to `SCRecorder.m`:
   - `AVCaptureDevice` for microphone
   - `AVCaptureAudioDataOutput` for audio frames
   - Add audio input to `AVAssetWriter`
   - Configure audio settings (AAC codec)

2. Audio format:
   - Codec: AAC
   - Sample rate: 48000 Hz
   - Channels: 1 (mono) or 2 (stereo)
   - Bitrate: 128 kbps

#### Steps
```objc
// 1. Get microphone device
AVCaptureDevice *audioDevice = [AVCaptureDevice defaultDeviceWithMediaType:AVMediaTypeAudio];

// 2. Create audio input
AVCaptureDeviceInput *audioInput = [AVCaptureDeviceInput deviceInputWithDevice:audioDevice error:&error];

// 3. Create audio writer input
NSDictionary *audioSettings = @{
    AVFormatIDKey: @(kAudioFormatMPEG4AAC),
    AVSampleRateKey: @(48000),
    AVNumberOfChannelsKey: @(1),
    AVEncoderBitRateKey: @(128000)
};
AVAssetWriterInput *audioWriterInput = [AVAssetWriterInput assetWriterInputWithMediaType:AVMediaTypeAudio outputSettings:audioSettings];

// 4. Add to AVAssetWriter
[assetWriter addInput:audioWriterInput];

// 5. Handle audio sample buffers in delegate
- (void)captureOutput:(AVCaptureOutput *)output didOutputSampleBuffer:(CMSampleBufferRef)sampleBuffer {
    if (audioWriterInput.readyForMoreMediaData) {
        [audioWriterInput appendSampleBuffer:sampleBuffer];
    }
}
```

### Phase 3: Update Rust API (30 min)
**Goal**: Expose audio capture in Rust wrapper

1. Add to FFI bridge (`bridge.rs`):
   ```rust
   extern "C" {
       pub fn sc_recorder_create_with_audio(
           output_path: *const c_char,
           width: u32,
           height: u32,
           fps: u32,
           quality: u32,
           display_id: u32,
           capture_audio: bool,
       ) -> *mut SCRecorder;
   }
   ```

2. Update `NativeRecorder::new()` to pass audio flag

### Phase 4: Integration with pulse-desktop (1 hour)
**Goal**: Wire up microphone toggle in UI

1. Update `src-tauri/src/capture/macos.rs`:
   ```rust
   let config = RecordingConfig {
       output_path: output_path.clone(),
       fps: 30,
       quality: 80,
       capture_cursor: true,
       capture_microphone: true,  // Read from app state
       display_id: Some(0),
       region: None,
   };
   ```

2. Read mic toggle state from `AppState`

### Phase 5: Testing (1 hour)
**Goal**: Verify audio capture works

1. **Test 1**: Record with mic ON
   - Press Cmd+Shift+R
   - Talk during recording
   - Release hotkey
   - Verify: `ffprobe` shows audio stream

2. **Test 2**: Record with mic OFF
   - Disable mic in UI
   - Record
   - Verify: `ffprobe` shows no audio stream

3. **Test 3**: Audio sync
   - Record 10 seconds
   - Clap at 2s, 5s, 8s
   - Verify: Audio sync matches video

## Technical Details

### Audio Codec (AAC)
```
Codec: AAC-LC (Low Complexity)
Sample Rate: 48000 Hz
Channels: 1 (mono)
Bitrate: 128 kbps
Container: MP4 (same as video)
```

### Permissions
macOS requires microphone permission:
```xml
<!-- Info.plist -->
<key>NSMicrophoneUsageDescription</key>
<string>Pulse Desktop needs microphone access to record audio.</string>
```

### Audio/Video Synchronization
- Both streams use same `AVAssetWriter`
- Timestamps aligned via `startSessionAtSourceTime:kCMTimeZero`
- Audio frames interleaved with video frames
- AVAssetWriter handles sync automatically

## File Size Impact

**Before (video-only)**:
- 3s recording: ~1.5 MB
- 5s recording: ~2.5 MB

**After (video + audio)**:
- 3s recording: ~1.6 MB (+100 KB for audio)
- 5s recording: ~2.6 MB (+100 KB for audio)

Audio overhead: ~128 kbps = 16 KB/s

## API Changes

### Before
```rust
let config = RecordingConfig {
    output_path: "recording.mp4".into(),
    fps: 30,
    quality: 80,
    ..Default::default()
};
```

### After
```rust
let config = RecordingConfig {
    output_path: "recording.mp4".into(),
    fps: 30,
    quality: 80,
    capture_microphone: true,  // NEW
    ..Default::default()
};
```

## Risks & Considerations

1. **Permission Prompts**:
   - macOS will prompt for mic permission
   - Need to handle denial gracefully

2. **Device Availability**:
   - What if no microphone?
   - Fail gracefully or continue without audio?

3. **Audio Latency**:
   - Microphone has inherent latency (~10-50ms)
   - AVAssetWriter should handle sync

4. **Background Noise**:
   - No noise cancellation in V0
   - User controls mic toggle

## Testing Checklist

- [ ] Mic permission prompt appears
- [ ] Mic permission denial handled gracefully
- [ ] Recording with mic ON includes audio
- [ ] Recording with mic OFF has no audio
- [ ] Audio sync matches video (±50ms)
- [ ] File size increases by ~16 KB/s
- [ ] ffprobe shows audio stream (AAC, 48kHz)
- [ ] Playback in QuickTime has audio
- [ ] No audio pops or clicks
- [ ] No memory leaks

## Timeline

**Total: 5-6 hours**
1. Config changes: 30 min
2. Objective-C implementation: 2-3 hours
3. Rust API: 30 min
4. Integration: 1 hour
5. Testing: 1 hour

## Next Steps

1. **Start with Phase 1**: Add `capture_microphone` to `RecordingConfig`
2. **Phase 2**: Implement Objective-C audio capture
3. **Test early**: Verify audio stream before full integration

---

**Note**: This plan focuses on **microphone only**. System audio capture is explicitly out of scope for V0 and would require:
- ScreenCaptureKit audio capture (macOS 13.0+)
- Different permissions
- Additional complexity
