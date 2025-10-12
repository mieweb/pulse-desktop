# MP4 Video Encoding Implementation Plan

## Current State (MVP)

**What We Have:**
- ✅ Screen capture at 30 FPS
- ✅ Frames stored as `Vec<Vec<u8>>` (raw PNG bytes in memory)
- ✅ Frame dimensions captured (width × height)
- ✅ Stop recording saves **last frame only** as PNG
- ✅ Sequential numbering: `recording_YYYYMMDD_HHMMSS.png`

**Limitations:**
- ❌ No video encoding (only last frame saved)
- ❌ Frames accumulate in memory (memory grows during recording)
- ❌ No MP4 output
- ❌ Duration calculation not implemented

---

## Goal: Encode All Frames to MP4

**Requirements:**
1. Convert raw frames to H.264-encoded video
2. Save as `.mp4` file with proper container
3. Sequential file naming: `recording-1.mp4`, `recording-2.mp4`, etc.
4. Calculate accurate duration from frame count
5. Reasonable file size (efficient compression)
6. Cross-platform (macOS first, Windows later)

---

## Research: Encoding Options

### Option 1: `ffmpeg-sys-next` + `ffmpeg-next` (Recommended)

**Pros:**
- Industry-standard encoding (H.264, AAC)
- Excellent quality/size ratio
- Mature, well-tested
- Supports all formats we need
- Good documentation

**Cons:**
- Requires FFmpeg installed on system
- Larger dependency graph
- Slightly complex API

**Example:**
```rust
use ffmpeg_next as ffmpeg;

// Initialize FFmpeg
ffmpeg::init()?;

// Create output file
let mut octx = ffmpeg::format::output(&output_path)?;

// Add video stream
let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264)?;
let mut video = octx.add_stream(codec)?;

// Set parameters
video.set_width(width);
video.set_height(height);
video.set_frame_rate((30, 1));
video.set_time_base((1, 30));

// Encode frames
for frame in frames {
    encoder.encode(&frame, &mut packet)?;
    octx.write_packet(&packet)?;
}
```

### Option 2: `mp4` crate (Simpler, Limited)

**Pros:**
- Pure Rust (no external dependencies)
- Simple API
- Lightweight

**Cons:**
- Requires pre-encoded H.264 bitstream (we'd need another encoder)
- No built-in encoding
- More manual work

### Option 3: `openh264` crate (Cisco's Encoder)

**Pros:**
- Pure Rust bindings
- Lightweight H.264 encoder
- No FFmpeg needed

**Cons:**
- H.264 encoding only (no container muxing)
- Still need MP4 container library
- Less mature than FFmpeg

---

## Decision: Use `ffmpeg-next`

**Rationale:**
- Best quality/performance balance
- Single library for encoding + muxing
- Future-proof (can add audio later)
- Well-documented patterns

**Trade-off Accepted:**
- Requires FFmpeg on system (check in docs)
- Larger dependency

---

## Implementation Strategy

### Phase 1: Add FFmpeg Dependencies

```toml
# Cargo.toml
[dependencies]
ffmpeg-next = "7.0"
ffmpeg-sys-next = "7.0"
```

**Note:** Requires FFmpeg installed:
- macOS: `brew install ffmpeg`
- Windows: Download from ffmpeg.org

### Phase 2: Convert PNG Frames to Raw RGB

**Current:** `Vec<Vec<u8>>` contains PNG-encoded bytes
**Needed:** Raw RGB/YUV pixels for FFmpeg

```rust
use image::io::Reader as ImageReader;

fn decode_frames(frames: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>, String> {
    frames.iter()
        .map(|png_bytes| {
            let img = ImageReader::new(Cursor::new(png_bytes))
                .with_guessed_format()?
                .decode()?;
            Ok(img.to_rgb8().into_raw())
        })
        .collect()
}
```

### Phase 3: Implement MP4 Encoder

**New file:** `src-tauri/src/encoding/mod.rs`

```rust
use ffmpeg_next as ffmpeg;
use std::path::PathBuf;

pub struct VideoEncoder {
    width: u32,
    height: u32,
    fps: u32,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        Self { width, height, fps }
    }

    pub fn encode_to_mp4(
        &self,
        frames: Vec<Vec<u8>>,
        output_path: PathBuf,
    ) -> Result<(), String> {
        // Initialize FFmpeg
        ffmpeg::init().map_err(|e| format!("FFmpeg init failed: {}", e))?;

        // Create output context
        let mut octx = ffmpeg::format::output(&output_path)
            .map_err(|e| format!("Failed to create output: {}", e))?;

        // Find H.264 encoder
        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264)
            .ok_or("H.264 encoder not found")?;

        // Add video stream
        let mut video = octx.add_stream(codec)
            .map_err(|e| format!("Failed to add stream: {}", e))?;

        // Configure encoder
        let mut encoder = video.codec().encoder().video()
            .map_err(|e| format!("Failed to get encoder: {}", e))?;

        encoder.set_width(self.width);
        encoder.set_height(self.height);
        encoder.set_aspect_ratio((self.width, self.height));
        encoder.set_format(ffmpeg::format::Pixel::RGB24);
        encoder.set_frame_rate(Some((self.fps as i32, 1)));
        encoder.set_time_base((1, self.fps as i32));

        // Open encoder
        let mut encoder = encoder.open_as(codec)
            .map_err(|e| format!("Failed to open encoder: {}", e))?;

        // Write header
        octx.write_header()
            .map_err(|e| format!("Failed to write header: {}", e))?;

        // Encode each frame
        for (i, frame_data) in frames.iter().enumerate() {
            let mut frame = ffmpeg::util::frame::video::Video::new(
                ffmpeg::format::Pixel::RGB24,
                self.width,
                self.height,
            );

            frame.data_mut(0).copy_from_slice(frame_data);
            frame.set_pts(Some(i as i64));

            encoder.send_frame(&frame)
                .map_err(|e| format!("Failed to send frame: {}", e))?;

            // Receive encoded packets
            let mut packet = ffmpeg::codec::packet::Packet::empty();
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(0);
                packet.rescale_ts(
                    (1, self.fps as i32),
                    video.time_base(),
                );
                octx.write_packet(&packet)
                    .map_err(|e| format!("Failed to write packet: {}", e))?;
            }
        }

        // Flush encoder
        encoder.send_eof()
            .map_err(|e| format!("Failed to send EOF: {}", e))?;

        let mut packet = ffmpeg::codec::packet::Packet::empty();
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(0);
            octx.write_packet(&packet)
                .map_err(|e| format!("Failed to write packet: {}", e))?;
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| format!("Failed to write trailer: {}", e))?;

        Ok(())
    }
}
```

### Phase 4: Update `stop_recording()` to Use Encoder

**File:** `src-tauri/src/capture/macos.rs`

```rust
pub async fn stop_recording(&mut self) -> Result<PathBuf, String> {
    // ... existing stop logic ...

    // Get frames and dimensions
    let frames = self.frames.lock().unwrap().clone();
    let (width, height) = self.frame_info.lock().unwrap()
        .ok_or("No frame info available")?;

    println!("🎬 Encoding {} frames to MP4...", frames.len());

    // Decode PNG frames to raw RGB
    let raw_frames: Vec<Vec<u8>> = frames.iter()
        .map(|png_bytes| {
            let img = image::load_from_memory(png_bytes)
                .map_err(|e| format!("Failed to decode frame: {}", e))?;
            Ok(img.to_rgb8().into_raw())
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Create encoder
    let encoder = VideoEncoder::new(width, height, 30);

    // Generate output path with sequential numbering
    let output_path = self.get_next_output_path();

    // Encode to MP4
    encoder.encode_to_mp4(raw_frames, output_path.clone())?;

    println!("✅ Video saved: {:?}", output_path);

    Ok(output_path)
}

fn get_next_output_path(&self) -> PathBuf {
    // Find highest existing recording-N.mp4
    let mut n = 1;
    loop {
        let path = self.output_path.join(format!("recording-{}.mp4", n));
        if !path.exists() {
            return path;
        }
        n += 1;
    }
}
```

### Phase 5: Sequential File Naming

**Pattern:** `recording-1.mp4`, `recording-2.mp4`, etc.

**Implementation:**
```rust
fn get_next_recording_number(output_folder: &PathBuf) -> u32 {
    let mut max_num = 0;
    
    if let Ok(entries) = std::fs::read_dir(output_folder) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                // Match pattern: recording-{N}.mp4
                if filename.starts_with("recording-") && filename.ends_with(".mp4") {
                    if let Some(num_str) = filename
                        .strip_prefix("recording-")
                        .and_then(|s| s.strip_suffix(".mp4"))
                    {
                        if let Ok(num) = num_str.parse::<u32>() {
                            max_num = max_num.max(num);
                        }
                    }
                }
            }
        }
    }
    
    max_num + 1
}
```

---

## Testing Plan

### Unit Tests
1. **Frame decoding**: PNG → RGB conversion
2. **Sequential numbering**: Correct N+1 logic
3. **Duration calculation**: `frame_count / fps`

### Integration Tests
1. **Short recording (5s)**: Verify MP4 playable
2. **Medium recording (30s)**: Check file size reasonable
3. **Empty recording (0 frames)**: Handle gracefully
4. **Multiple recordings**: Sequential numbering works

### Manual Tests
1. **Press/release quickly**: 1-2 frame video
2. **Hold for 10 seconds**: ~300 frames
3. **Check with QuickTime**: Verify playback
4. **Check with `ffprobe`**: Verify codec/format

```bash
# Verify output
ffprobe recording-1.mp4

# Expected output:
# Duration: 00:00:10.00
# Stream #0:0: Video: h264, 1920x1080, 30 fps
```

---

## Implementation Steps (Next Actions)

1. ✅ Create this plan document
2. ⏳ Add `ffmpeg-next` dependency to Cargo.toml
3. ⏳ Create `src-tauri/src/encoding/mod.rs`
4. ⏳ Implement `VideoEncoder` struct
5. ⏳ Update `stop_recording()` to decode frames + encode MP4
6. ⏳ Implement sequential file naming
7. ⏳ Add duration calculation
8. ⏳ Test with short recording
9. ⏳ Test with multiple recordings
10. ⏳ Update DEVELOPMENT_PROGRESS.md

---

## Expected Outcome

**Before:**
- Output: `~/Movies/PushToHold/recording_20251012_143022.png` (last frame only)
- Size: ~2 MB
- Duration: N/A

**After:**
- Output: `~/Movies/PushToHold/recording-1.mp4`
- Size: ~5-10 MB for 30s @ 1920×1080
- Duration: Accurate (matches hold time)
- Format: H.264 video, 30 fps, playable in QuickTime/VLC

---

## Risks & Mitigation

**Risk 1: FFmpeg not installed**
- Mitigation: Check at runtime, provide clear error message
- Documentation: Add FFmpeg requirement to README

**Risk 2: Memory usage during encoding**
- Current: All frames in memory (PNG)
- After: Decode to RGB (larger!), then encode
- Mitigation: Process in batches? (future optimization)

**Risk 3: Encoding takes time**
- 300 frames × encoding overhead = several seconds?
- Mitigation: Show "Encoding..." status in UI
- Keep background thread (don't block UI)

**Risk 4: Cross-platform FFmpeg**
- macOS: brew install works
- Windows: May need bundled DLLs
- Mitigation: Document per-platform setup

---

## Future Enhancements (Not in This Iteration)

- [ ] GPU-accelerated encoding (VideoToolbox on macOS)
- [ ] Configurable bitrate/quality
- [ ] Real-time encoding (stream to disk)
- [ ] Audio mixing (when mic feature added)
- [ ] Progress bar during encoding

---

**Status:** ✅ Plan Complete, Ready to Implement
**Next Step:** Add FFmpeg dependencies + create encoder module
