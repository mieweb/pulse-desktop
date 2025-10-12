# Testing Screen Capture

## Prerequisites

1. **macOS Screen Recording Permission:**
   - System Settings → Privacy & Security → Screen Recording
   - Pulse Desktop may need to be added to the allowed list
   - The app will request permission on first capture attempt

2. **App Running:**
   ```bash
   deno task tauri dev
   ```

## Test Procedure

### Test 1: Basic Capture
1. Launch the app
2. Press and hold **Cmd+Shift+R** for 2-3 seconds
3. Release the key
4. Check console for output

**Expected Console Output:**
```
🎬 Starting recording...
🎬 Starting screen capture...
📺 Capturing display: 1 (3024x1964)
📸 Captured 30 frames (3024x1964)
📸 Captured 60 frames (3024x1964)
⏹️  Stopping recording...
⏹️  Stopping screen capture...
🛑 Capture thread received stop signal
✅ Capture thread finished with 87 frames
📊 Recording complete:
  Duration: 2.93s
  Frames: 87
  FPS: 29.69
💾 Saved last frame to: "/Users/you/Movies/PushToHold/recording_20251011_143052.png"
```

**Expected UI:**
- Status changes: Idle → Recording → Idle
- Clip counter increments
- Success message with file path

**Expected File:**
- Location: `~/Movies/PushToHold/recording_YYYYMMDD_HHMMSS.png`
- Format: PNG image
- Content: Last frame of your screen
- Size: ~2-5 MB (depends on resolution)

### Test 2: Rapid Recording
1. Press Cmd+Shift+R (hold 1 second)
2. Release
3. Immediately press again (hold 1 second)
4. Release
5. Repeat 3-4 times quickly

**Expected Behavior:**
- No crashes
- Each recording creates a new file
- Clip counter increments each time
- No "Already recording" errors
- Status transitions smoothly

### Test 3: Duration Accuracy
1. Use a stopwatch
2. Press and hold Cmd+Shift+R for exactly 5 seconds
3. Release
4. Check console for reported duration

**Expected:**
- Duration: 4.85 - 5.15 seconds (±150ms tolerance)
- Frame count: ~145-155 frames (30 FPS × 5 sec)
- FPS: 28-32 FPS

### Test 4: Error Handling
**Test 4a: Permission Denied**
1. Go to System Settings → Privacy & Security → Screen Recording
2. Remove Pulse Desktop from allowed apps
3. Try to record

**Expected:**
- Error message in UI
- Console shows "Failed to start recording" error
- Status returns to Idle
- No crash

**Test 4b: Invalid Output Folder**
1. Set output folder to `/invalid/path/that/does/not/exist`
2. Try to record

**Expected:**
- Folder is created automatically (if permissions allow)
- OR error message about folder creation failure

### Test 5: Memory Usage
1. Record for 30 seconds (long hold)
2. Check Activity Monitor during recording

**Expected:**
- Memory grows during recording (~5-10 MB/sec)
- Memory released after recording stops
- No memory leaks on repeated recordings

### Test 6: Cross-Application Hotkey
1. Open Chrome (or any other app)
2. Make Chrome the active window
3. Press Cmd+Shift+R (hold 2 seconds)
4. Release

**Expected:**
- Recording works even when Pulse Desktop is not active
- Captures the screen (Chrome window visible in capture)
- Status updates in Pulse Desktop window

## Verification Checklist

- [ ] PNG file created in correct folder
- [ ] File timestamp matches recording time
- [ ] Image opens in Preview.app
- [ ] Image shows screen content at time of recording
- [ ] Clip counter increments
- [ ] Success message displays file path
- [ ] Console shows FPS ~29-30
- [ ] Console shows accurate duration
- [ ] No crashes during recording
- [ ] No crashes on rapid press/release
- [ ] Memory released after recording
- [ ] Works with other apps in foreground

## Known Issues (MVP)

⚠️ **Expected Limitations:**
1. Only last frame saved (not full video)
2. Memory grows during recording (frames in RAM)
3. No microphone audio yet
4. No region selection yet
5. Cannot save as MP4 yet

⚠️ **Not Bugs:**
- "MP4 encoding not yet implemented" message - This is expected
- Saves PNG instead of MP4 - This is MVP behavior
- Memory usage grows - Frames stored in RAM for future encoding

## Troubleshooting

### Issue: "Failed to get screens"
**Solution:** Grant Screen Recording permission in System Settings

### Issue: "Failed to create output folder"
**Solution:** Check folder permissions, try default folder ~/Movies/PushToHold

### Issue: Status stuck on "Recording"
**Solution:** 
1. Check console for errors
2. Restart app
3. May indicate capture thread crashed

### Issue: No file created
**Solution:**
1. Check console for "Failed to write PNG" error
2. Verify output folder exists and is writable
3. Check disk space

### Issue: Low FPS (<20)
**Solution:**
- Normal on high-resolution displays
- Will improve with MP4 encoding optimization

## Debugging

**Enable Verbose Logging:**
```bash
# Already enabled in current implementation
# Check console for detailed capture logs
```

**Check Capture Thread:**
- Look for "Capture thread finished" message
- If missing, thread may have crashed
- Check for "Error capturing frame" messages

**Verify Permissions:**
```bash
# Check if app has screen recording permission
tccutil query com.apple.screencapture $(bundle_id)
```

## Next Testing Phase

After MP4 encoding is implemented:
1. Test video playback
2. Test audio sync (when mic added)
3. Test different resolutions
4. Test region capture
5. Test scaling options

---

**Last Updated:** October 11, 2025
**Feature Status:** MVP Complete, PNG output only
**Next:** MP4 encoding
