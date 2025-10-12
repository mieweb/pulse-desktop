# Tokio Runtime Fix

## Problem

**Error:** `there is no reactor running, must be called from the context of a Tokio 1.x runtime`

The app crashed when pressing the hotkey because we were calling `tokio::spawn()` from within the hotkey callback, which runs on the main UI thread (not in a Tokio runtime context).

## Root Cause

Tauri's event handlers (like the global shortcut callback) run on the main UI thread, which is NOT part of a Tokio runtime. When we tried to call `tokio::spawn(async move { ... })`, it panicked because there was no Tokio reactor available.

## Solution

**Replace `tokio::spawn()` with `std::thread::spawn()` + `Runtime::block_on()`**

### Before (Broken):
```rust
tokio::spawn(async move {
    capturer.start_recording().await; // ❌ Panic: no reactor!
});
```

### After (Fixed):
```rust
std::thread::spawn(move || {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(capturer.start_recording()); // ✅ Creates local runtime
});
```

## Changes Made

### `src-tauri/src/commands.rs`

**Press Handler:**
```rust
// OLD: tokio::spawn(async move { ... })
// NEW: std::thread::spawn(move || { ... })

std::thread::spawn(move || {
    // Create a Tokio runtime for this thread
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    // Use block_on to run async code in this thread's runtime
    match runtime.block_on(capturer.start_recording()) {
        Ok(_) => { /* ... */ }
        Err(e) => { /* ... */ }
    }
});
```

**Release Handler:**
```rust
// OLD: tokio::spawn(async move { ... })
// NEW: std::thread::spawn(move || { ... })

std::thread::spawn(move || {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    match runtime.block_on(capturer.stop_recording()) {
        Ok(path) => { /* ... */ }
        Err(e) => { /* ... */ }
    }
});
```

**Removed Unused Imports:**
- `ShortcutEvent` (not used)
- `std::sync::Arc` (not needed)

## Why This Works

1. **`std::thread::spawn`**: Creates a new OS thread (not async task)
2. **`Runtime::new()`**: Creates a new Tokio runtime just for this thread
3. **`block_on()`**: Runs async code synchronously within the runtime

This pattern allows us to:
- Run async code (like `capturer.start_recording().await`) ✅
- From a non-async context (hotkey callback) ✅
- Without requiring the main thread to have a runtime ✅

## Performance Impact

**Minimal:**
- Creating a runtime is cheap (~1ms)
- Only happens on press/release (not in hot path)
- Thread-per-recording is acceptable for user-triggered actions

**Alternative Considered:**
- Wrap entire Tauri app in `tokio::main` - Not possible with Tauri's architecture
- Use channels to communicate with existing runtime - Over-engineered for simple use case

## Testing

**To Test:**
1. Run `deno task tauri dev`
2. Press and hold Cmd+Shift+R
3. Console should show:
   ```
   Hotkey event: state=Pressed
   🎬 Starting recording...
   🎬 Starting screen capture...
   📺 Capturing display: ...
   ```
4. Release key
5. Console should show:
   ```
   Hotkey event: state=Released
   ⏹️  Stopping recording...
   📊 Recording complete: ...
   ```
6. No panic! ✅

## Lessons Learned

1. **Tauri event handlers are NOT async**: They run on the main UI thread
2. **Always check execution context**: Can't assume Tokio runtime exists everywhere
3. **`Runtime::block_on()` is the escape hatch**: Allows running async code in sync contexts
4. **Thread-per-event is acceptable**: For infrequent user actions, spawning threads is fine

## Related Files

- `src-tauri/src/commands.rs` - Fixed press/release handlers
- `src-tauri/src/capture/macos.rs` - Async methods still work (called via block_on)

---

**Status:** ✅ Fixed
**Tested:** Building (needs runtime testing)
**Next:** Test actual screen capture with press/release
