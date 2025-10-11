# Race Condition Fix: Status Display Issue

## Problem
When pressing Cmd+Shift+R rapidly (press → release → press again quickly), the status chip would show "Idle" even though recording was active. The console would correctly show "🎬 Starting recording..." but the UI would be out of sync.

## Root Cause
The issue was a race condition in the event flow:

### Original Flow (Broken)
```
Press   → emit("recording") → [recording state active]
Release → emit("saving")
        → spawn thread (500ms delay)
            → emit("idle")      ← This arrives LATE
            → emit("clip-saved")

Press (2nd time) → emit("recording")  ← Arrives BEFORE the delayed "idle"
```

The delayed `emit("idle")` from the first recording cycle would arrive **after** the second `emit("recording")`, causing the UI to incorrectly show "Idle" during active recording.

## Solution
Remove the "saving" intermediate state and emit "idle" **immediately** when recording stops:

### Fixed Flow
```
Press   → emit("recording") → [recording state active]
Release → emit("idle") [IMMEDIATELY]  ← No delay!
        → spawn thread (500ms delay)
            → emit("clip-saved")  ← Only for file notification

Press (2nd time) → emit("recording")  ← Always arrives AFTER idle
```

## Code Changes

### Backend (src-tauri/src/commands.rs)
```rust
ShortcutState::Released => {
    if IS_RECORDING.swap(false, Ordering::SeqCst) {
        println!("⏹️  Stopping recording...");
        
        // ✅ Immediately transition to idle to allow rapid re-recording
        let _ = events::emit_status(app, "idle");
        
        // Background save happens separately
        let app_clone = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = events::emit_clip_saved(&app_clone, /* ... */);
        });
    }
}
```

### Frontend (src/hooks/useRecording.ts)
```typescript
// Listen for clip saved events
const unlistenClipSaved = listen<ClipSavedEvent>('clip-saved', (event) => {
  setRecordingState((prev) => ({
    ...prev,
    // ✅ Don't change status - already set by recording-status event
    clipCount: prev.clipCount + 1,
    currentClipPath: event.payload.path,
  }));
});
```

## Benefits
1. **Instant feedback**: UI shows "Idle" immediately when you release the key
2. **No race conditions**: Status transitions happen synchronously
3. **Rapid recording**: Can start a new recording immediately without waiting for file save
4. **Correct state**: Background file saving doesn't interfere with status display

## Testing
- ✅ Press and hold → Release → Status shows "Recording" → "Idle" correctly
- ✅ Rapid press/release cycles → Status always correct
- ✅ File save notification arrives later without affecting status

## Design Decision
We removed the "saving" status entirely because:
1. File saves happen in background (non-blocking)
2. Users should be able to start new recordings immediately
3. The "clip-saved" event provides enough feedback about file completion
4. Simpler state machine = fewer race conditions
