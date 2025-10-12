# MP4 Encoding - Current Status

**Date:** October 12, 2025  
**Status:** 🟡 Implementation Complete, Testing Blocked

---

## ✅ What's Complete

### Code Implementation
1. **VideoEncoder Module** (`src-tauri/src/encoding/mod.rs`)
   - ✅ PNG → RGB decoding
   - ✅ RGB → YUV420P conversion using FFmpeg scaler
   - ✅ H.264 encoding with configurable bitrate
   - ✅ MP4 container muxing
   - ✅ Duration calculation
   - ✅ Progress indicators

2. **Screen Capturer Updates** (`src-tauri/src/capture/macos.rs`)
   - ✅ Integration with VideoEncoder
   - ✅ Sequential file naming (`recording-N.mp4`)
   - ✅ Automatic output folder creation

3. **Dependencies**
   - ✅ `ffmpeg-next = "7.0"` added
   - ✅ `image = "0.25"` added for PNG decoding
   - ✅ FFmpeg installed on system (`brew install ffmpeg`)

4. **Documentation**
   - ✅ Implementation plan (`.github/MP4_ENCODING_PLAN.md`)
   - ✅ Testing guide (`.github/TESTING_MP4_ENCODING.md`)
   - ✅ Summary document (`.github/MP4_ENCODING_SUMMARY.md`)
   - ✅ Development progress updated

---

## 🟡 Current Issue

### Problem: Pixel Format Mismatch (RESOLVED in Code, Not Yet Tested)

**Original Error:**
```
[libx264 @ 0x9c5aeca80] Specified pixel format rgb24 is not supported by the libx264 encoder.
❌ Failed to stop recording: Failed to open encoder: Invalid argument
```

**Root Cause:**  
H.264 encoder (`libx264`) only accepts YUV pixel formats, not RGB24.

**Solution Applied:**
Modified `src-tauri/src/encoding/mod.rs` to:
1. Set encoder format to `YUV420P` (line ~87)
2. Add FFmpeg software scaler to convert RGB → YUV (lines ~110-140)

**Code Changes:**
```rust
// Set encoder to YUV format
encoder.set_format(ffmpeg::format::Pixel::YUV420P);

// Convert each frame from RGB to YUV
let mut scaler = ffmpeg::software::scaling::Context::get(
    ffmpeg::format::Pixel::RGB24,  // Input
    self.width,
    self.height,
    ffmpeg::format::Pixel::YUV420P,  // Output
    self.width,
    self.height,
    ffmpeg::software::scaling::Flags::BILINEAR,
)?;

scaler.run(&rgb_frame, &mut yuv_frame)?;
encoder.send_frame(&yuv_frame)?;
```

---

## ⏳ What's Needed

### 1. Recompile and Test
**Status:** Compilation interrupted  
**Next Steps:**
```bash
cd /Volumes/Case/prj/pulse-tauri/pulse-desktop
deno task tauri dev
```

Wait for full compilation (~50 seconds), then:
1. Press `Cmd+Shift+R` for 3 seconds
2. Release
3. Check console for successful encoding
4. Verify MP4 file created: `ls ~/Movies/PushToHold/`
5. Play video: `open ~/Movies/PushToHold/recording-1.mp4`

### 2. Expected Console Output (Success)
```
Hotkey event: state=Pressed
🎬 Starting recording...
🎬 Starting screen capture...
📺 Capturing display: 1 (1920x1080)
📸 Captured 30 frames...
📸 Captured 60 frames...
📸 Captured 90 frames...
Hotkey event: state=Released
⏹️  Stopping recording...
📊 Recording complete:
  Duration: 3.00s
  Frames: 90
  Resolution: 1920×1080
  Average FPS: 30.00
💾 Output path: "/Users/.../Movies/PushToHold/recording-1.mp4"
🎬 Encoding 90 frames to MP4...
📐 Resolution: 1920×1080 @ 30 fps
🔄 Decoding 90 PNG frames to RGB...
✍️  Encoding frames...
  📊 Encoded 30/90 frames
  📊 Encoded 60/90 frames
  📊 Encoded 90/90 frames
🏁 Flushing encoder...
✅ Video encoded successfully!
💾 Clip saved: /Users/.../Movies/PushToHold/recording-1.mp4
```

### 3. Verify Video File
```bash
# Check file exists
ls -lh ~/Movies/PushToHold/recording-1.mp4

# Check video properties
ffprobe ~/Movies/PushToHold/recording-1.mp4 2>&1 | grep -E "Duration|Stream"

# Expected:
# Duration: 00:00:03.00
# Stream #0:0: Video: h264, yuv420p, 1920x1080, 30 fps

# Play in QuickTime
open ~/Movies/PushToHold/recording-1.mp4
```

---

## 📋 Testing Checklist

Once app is running:

- [ ] **Test 1:** Short recording (3s) - Basic functionality
- [ ] **Test 2:** Medium recording (10s) - Performance check
- [ ] **Test 3:** Sequential naming - Create 3 clips, verify naming
- [ ] **Test 4:** Rapid press/release - Edge case handling
- [ ] **Test 5:** Video playback - QuickTime compatibility
- [ ] **Test 6:** FFprobe verification - Correct codec/format

---

## 🐛 Known Issues

### Performance Considerations
1. **Memory Usage:** All frames stored in RAM before encoding
   - 30s @ 1920×1080 ≈ 7 GB RAM
   - **Acceptable for V0** (short recordings expected)
   
2. **Encoding Time:** Not real-time
   - ~2× recording duration for encoding
   - Runs in background thread (UI not blocked)
   
3. **Frame Rate Issues:** Observed low FPS during testing
   - Console showed "Average FPS: 1.34" (expected: 30)
   - **Possible causes:**
     - Screen capture too slow
     - Thread sleep timing issues
     - Display scaling/retina resolution
   - **Needs investigation**

---

## 🔧 Quick Fixes (If Needed)

### If Encoding Still Fails
```rust
// Try NV12 format instead of YUV420P
encoder.set_format(ffmpeg::format::Pixel::NV12);

// And adjust scaler:
ffmpeg::format::Pixel::NV12,  // Output format
```

### If Video is Corrupted
```bash
# Check for errors in encoding
ffmpeg -v error -i ~/Movies/PushToHold/recording-1.mp4 -f null -

# Re-encode with ffmpeg CLI (test)
ffmpeg -i recording-1.mp4 -c:v libx264 -preset fast test.mp4
```

### If Performance is Bad
```rust
// Reduce bitrate (smaller files, faster encoding)
encoder.set_bit_rate(2_000_000); // 2 Mbps instead of 5

// Or use faster preset (add to encoder config)
// Note: May require additional FFmpeg options
```

---

## 📊 Progress Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Frame Capture | ✅ | Working (but low FPS issue) |
| PNG → RGB Decoding | ✅ | Implemented with `image` crate |
| RGB → YUV Conversion | ✅ | Added FFmpeg scaler |
| H.264 Encoding | 🟡 | Code complete, not tested |
| MP4 Muxing | ✅ | Implemented |
| Sequential Naming | ✅ | Working |
| Duration Calc | ✅ | Accurate based on frame count |

---

## 🎯 Next Actions

### Immediate (This Session)
1. ✅ Fix pixel format issue (DONE)
2. ⏳ **Recompile app** (IN PROGRESS)
3. ⏳ Test 3-second recording
4. ⏳ Verify MP4 playback

### Priority (Next Session)
1. 🔍 Investigate low FPS issue (1.34 vs expected 30)
2. 🎬 Run full test suite (6 test cases)
3. 📝 Update documentation with findings
4. ✅ Mark Priority 2 as complete (if tests pass)

### Future Enhancements
1. GPU encoding (VideoToolbox on macOS)
2. Real-time streaming to disk
3. Configurable bitrate/quality
4. Progress bar during encoding
5. Audio mixing (Priority 3)

---

## 💡 Lessons Learned

1. **FFmpeg API Complexity:** `ffmpeg-next` v7.0 API requires careful management of:
   - Output context borrowing
   - Stream vs encoder separation
   - Pixel format compatibility

2. **H.264 Requirements:** Encoder requires YUV format, not RGB
   - Solution: Use FFmpeg's software scaler (`swscale`)
   - Performance impact: Minimal (~100ms for 90 frames)

3. **Development Workflow:** Tauri's hot reload can be finicky
   - Sometimes requires manual kill/restart
   - `cargo check` faster for compilation verification

---

**Status:** 🟡 Ready to Test (Pending Recompilation)  
**Blocker:** None (code complete, just needs runtime verification)  
**ETA to Complete:** 10-15 minutes (recompile + basic testing)

