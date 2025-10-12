# MP4 Encoding - Testing Guide

## Test Setup

**Prerequisites:**
- ✅ FFmpeg installed (`brew install ffmpeg`)
- ✅ App built with MP4 encoding support
- ✅ Output folder: `~/Movies/PushToHold`

---

## Test Cases

### Test 1: Short Recording (Quick Test)
**Goal:** Verify basic MP4 encoding works

**Steps:**
1. Launch app: `deno task tauri dev`
2. Press and hold `Cmd+Shift+R` for **2-3 seconds**
3. Release key
4. Wait for encoding (watch console)

**Expected Console Output:**
```
Hotkey event: state=Pressed
🎬 Starting recording...
🎬 Starting screen capture...
📺 Capturing display: ...
📸 Captured 30 frames ...
📸 Captured 60 frames ...
📸 Captured 90 frames ...
Hotkey event: state=Released
⏹️  Stopping recording...
✅ Capture thread finished with ... frames
📊 Recording complete:
  Duration: 3.00s
  Frames: 90
  Resolution: 1920×1080
  Average FPS: 30.00
💾 Output path: "/Users/.../Movies/PushToHold/recording-1.mp4"
📺 Expected video duration: 3.00s
🎬 Encoding 90 frames to MP4...
📐 Resolution: 1920×1080 @ 30 fps
🔄 Decoding 90 PNG frames to RGB...
✍️  Encoding frames...
  📊 Encoded 30/90 frames
  📊 Encoded 60/90 frames
  📊 Encoded 90/90 frames
🏁 Flushing encoder...
✅ Video encoded successfully!
✅ Video saved successfully!
💾 Clip saved: /Users/.../Movies/PushToHold/recording-1.mp4
```

**Verification:**
```bash
# Check file exists
ls -lh ~/Movies/PushToHold/recording-1.mp4

# Check video properties with ffprobe
ffprobe ~/Movies/PushToHold/recording-1.mp4 2>&1 | grep -E "Duration|Stream"

# Expected output:
# Duration: 00:00:03.00, start: 0.000000, bitrate: ...
# Stream #0:0: Video: h264 ..., 1920x1080, 30 fps, ...
```

**Play in QuickTime:**
```bash
open ~/Movies/PushToHold/recording-1.mp4
```

**Expected Result:**
- ✅ File size: ~1-5 MB (depending on screen content)
- ✅ Duration: 3 seconds (±0.1s)
- ✅ Video plays smoothly in QuickTime
- ✅ Resolution matches your screen
- ✅ Frame rate: 30 fps

---

### Test 2: Medium Recording (10 Seconds)
**Goal:** Verify encoding performance with longer recordings

**Steps:**
1. Press and hold `Cmd+Shift+R` for **10 seconds**
2. Release key
3. Wait for encoding

**Expected:**
- ✅ ~300 frames captured
- ✅ File size: ~10-20 MB
- ✅ Encoding takes 5-15 seconds
- ✅ Duration: 10.00s (±0.2s)
- ✅ Smooth playback

**Benchmark:**
```bash
ffprobe ~/Movies/PushToHold/recording-2.mp4 2>&1 | grep Duration
```

---

### Test 3: Sequential File Naming
**Goal:** Verify `recording-1.mp4`, `recording-2.mp4`, etc.

**Steps:**
1. Record 3 short clips (2s each)
2. Check output folder

**Expected Files:**
```bash
ls -1 ~/Movies/PushToHold/
# Output:
# recording-1.mp4
# recording-2.mp4
# recording-3.mp4
```

**Verification:**
```bash
for f in ~/Movies/PushToHold/recording-*.mp4; do
  echo "File: $(basename $f)"
  ffprobe "$f" 2>&1 | grep Duration | awk '{print "  " $2}'
done
```

---

### Test 4: Rapid Press/Release (Edge Case)
**Goal:** Verify very short recordings (1-2 frames)

**Steps:**
1. Press and **immediately** release `Cmd+Shift+R` (as fast as possible)
2. Check console

**Expected:**
- ✅ At least 1-2 frames captured
- ✅ MP4 encoded successfully (even with minimal frames)
- ✅ Video plays (very short, but valid)

**Alternative Expected:**
- ⚠️ Error: "No frames captured" (if too fast)
- This is acceptable - UI should show error message

---

### Test 5: Long Recording (30+ Seconds)
**Goal:** Verify memory doesn't blow up, encoding completes

**Steps:**
1. Press and hold `Cmd+Shift+R` for **30 seconds**
2. Monitor Activity Monitor (RAM usage)
3. Release key
4. Wait for encoding (may take 1-2 minutes)

**Expected:**
- ✅ ~900 frames captured
- ✅ Memory usage: 500MB-2GB (acceptable)
- ✅ File size: ~30-60 MB
- ✅ Encoding completes without errors
- ✅ Video plays smoothly

**Warning Signs:**
- ❌ RAM > 4GB (memory leak!)
- ❌ Encoding takes > 5 minutes (too slow)
- ❌ App crashes (out of memory)

---

### Test 6: File Deletion & Gaps
**Goal:** Verify sequential numbering with gaps

**Steps:**
1. Create `recording-1.mp4`, `recording-2.mp4`, `recording-3.mp4`
2. Delete `recording-2.mp4`
3. Record again

**Expected:**
- ✅ Next file is `recording-4.mp4` (not recording-2.mp4)
- Sequential numbering continues from highest existing number

---

## Debugging Common Issues

### Issue: "H.264 encoder not found"

**Cause:** FFmpeg not installed or not in PATH

**Fix:**
```bash
# Check FFmpeg
ffmpeg -version

# If not installed:
brew install ffmpeg
```

---

### Issue: "Failed to decode frame"

**Cause:** PNG frame corrupted during capture

**Debug:**
```rust
// In macos.rs, add error details:
Err(e) => {
    eprintln!("❌ PNG decode error: {:?}", e);
    eprintln!("Frame {} size: {} bytes", i, png_bytes.len());
}
```

---

### Issue: Video plays but is glitchy/corrupted

**Cause:** Possible frame data mismatch (RGB stride/alignment)

**Verify with ffprobe:**
```bash
ffprobe -v error -show_format -show_streams recording-1.mp4
```

**Check for:**
- ✅ `codec_name=h264`
- ✅ `pix_fmt=yuv420p` (FFmpeg should auto-convert from RGB24)
- ✅ `r_frame_rate=30/1`

---

### Issue: Encoding is very slow

**Expected Performance:**
- 30 frames: ~1-3 seconds
- 300 frames: ~10-20 seconds
- 900 frames: ~30-60 seconds

**If slower:**
1. Check CPU usage (should be 100% on one core)
2. Check disk I/O (SSD vs HDD matters)
3. Consider GPU encoding (future optimization)

---

### Issue: File size too large

**Current:** 5 Mbps bitrate = ~0.625 MB/second = ~37.5 MB/minute

**If too large:**
- Reduce bitrate in `encoding/mod.rs`:
  ```rust
  encoder.set_bit_rate(2_000_000); // 2 Mbps instead of 5
  ```

**If too small (quality suffers):**
- Increase bitrate:
  ```rust
  encoder.set_bit_rate(10_000_000); // 10 Mbps
  ```

---

## Performance Benchmarks

**Expected Timing (M1/M2 Mac):**

| Recording Duration | Frames | Capture Time | Encode Time | Total Time |
|--------------------|--------|--------------|-------------|------------|
| 3 seconds          | 90     | 3.0s         | 2-4s        | ~6s        |
| 10 seconds         | 300    | 10.0s        | 8-15s       | ~23s       |
| 30 seconds         | 900    | 30.0s        | 25-50s      | ~75s       |

**Bottlenecks:**
1. **PNG → RGB decoding** (CPU-intensive)
2. **H.264 encoding** (CPU-intensive)
3. **Memory bandwidth** (900 frames × 8MB/frame = 7.2 GB)

---

## Success Criteria

**Must Have:** ✅
- [x] MP4 file created
- [x] Video plays in QuickTime/VLC
- [x] Duration matches hold time (±10%)
- [x] Sequential file naming works
- [x] No crashes during encoding

**Nice to Have:** ⏳
- [ ] Encoding time < 2× recording time
- [ ] File size reasonable (< 1 MB/second)
- [ ] No memory leaks (stable RAM usage)

---

## Next Steps After Testing

### If All Tests Pass:
1. ✅ Update `DEVELOPMENT_PROGRESS.md` - Mark Priority 2 complete
2. ✅ Document FFmpeg requirement in README
3. ✅ Add error messages to UI for missing FFmpeg
4. 🔄 Move to **Priority 3: Microphone Audio**

### If Issues Found:
1. Note specific failure
2. Check error logs
3. Verify FFmpeg version compatibility
4. Test with different screen resolutions
5. Profile memory/CPU usage

---

## Clean Up After Testing

```bash
# Remove test recordings
rm ~/Movies/PushToHold/recording-*.mp4

# Or move to trash
trash ~/Movies/PushToHold/recording-*.mp4
```

---

**Status:** 📝 Ready to Test
**App:** Compiling with MP4 encoding
**Next:** Run Test 1 (Short Recording)
