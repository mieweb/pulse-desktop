# Logging Implementation

## Overview

Replaced all `println!` and `eprintln!` statements throughout the codebase with proper structured logging using the `log` and `env_logger` crates. The logging system includes automatic delta time tracking between log messages for performance monitoring.

## Changes Made

### 1. Added Dependencies (`src-tauri/Cargo.toml`)
```toml
log = "0.4"
env_logger = "0.11"
```

### 2. Created Logging Module (`src-tauri/src/logging.rs`)

Custom logger initialization with:
- **Delta time tracking**: Shows milliseconds elapsed between log messages
- **Timestamp formatting**: ISO 8601 format with milliseconds
- **Log levels**: Debug, Info, Warn, Error
- **Format**: `YYYY-MM-DDTHH:MM:SS.mmm [+XXX ms] [LEVEL] - message`

Example output:
```
2025-10-26T14:23:45.123 [+0 ms] [INFO] - Starting app
2025-10-26T14:23:45.243 [+120 ms] [WARN] - Slow response detected
2025-10-26T14:23:45.323 [+80 ms] [ERROR] - Network error
```

### 3. Updated All Source Files

#### `src-tauri/src/commands.rs`
- **Debug level**: Hotkey events, UI actions, region selector operations
- **Info level**: Recording lifecycle (start/stop), successful operations, status updates
- **Warn level**: Expected issues (no project selected, already recording, re-initialization failures)
- **Error level**: Critical failures (slow starts >100ms, recording errors, timeline failures)

Key logging points:
- **Pre-initialization**: `"⚡ Pre-initializing capturer for project: {}"`
- **Fast path**: `"⚡ Using pre-initialized capturer (fast path)"`
- **Slow path**: `"🐌 SLOW PATH: Creating capturer on demand"` (ERROR level)
- **Slow start detection**: `"⚠️ SLOW START DETECTED: {:?}"` (ERROR level with apology)
- **Recording lifecycle**: Start, stop, saved, re-initialization

#### `src-tauri/src/fs_watcher.rs`
- **Info level**: Watcher control (pause/resume), successful startup
- **Debug level**: Detailed filesystem events (when not paused), path changes

Key logging points:
- **Pause**: `"⏸️ Pausing filesystem watcher"`
- **Resume**: `"▶️ Resuming filesystem watcher"`
- **Event received but paused**: `"📂 Filesystem event received but PAUSED"` (DEBUG)
- **Event details**: Path changes, video file detection, directory changes

#### `src-tauri/src/lib.rs`
- **Info level**: Watcher startup success
- **Warn level**: Watcher startup failure

#### `src-tauri/src/capture/macos.rs`
- **Info level**: Pre-initialization start/complete, recording start/complete, saved files
- **Debug level**: Output paths, permission requests, recording details
- **Error level**: Slow starts >100ms (with apology message)

Key logging points:
- **Pre-init start**: `"🚀 Pre-initializing ScreenCaptureKit (this takes 2-3 seconds)..."`
- **Pre-init complete**: `"✅ ScreenCaptureKit pre-initialized in {:?}"`
- **Recording start**: `"▶️ Recording started in {:?}"`
- **Slow start**: `"⚠️ SLOW START: {:?}"` + apology message (ERROR)
- **Recording complete**: `"📊 Recording complete:"` with duration stats

#### `src-tauri/src/hotkey/{macos,windows}.rs`
- **Debug level**: Hotkey registration/unregistration

#### `src-tauri/src/capture/windows.rs`
- **Debug level**: Recording start/stop operations

### 4. Log Level Guidelines

**DEBUG**: Detailed operational information for troubleshooting
- Hotkey events
- Filesystem event details
- Path operations
- Region selector operations

**INFO**: Normal operational milestones
- Recording lifecycle events
- Successful operations
- Status changes
- Performance metrics (when within acceptable range)

**WARN**: Expected issues that don't stop operation
- No project selected
- Already recording
- Re-initialization failures
- Missing resources

**ERROR**: Critical failures requiring attention
- Recording startup >100ms (with apology)
- Capturer not pre-initialized
- Recording failures
- Save errors
- Timeline failures

## Performance Monitoring

The delta time logging automatically tracks performance:

```rust
info!("⚡ Pre-initializing capturer");  // [+0 ms]
// ... 2-3 seconds of initialization ...
info!("✅ Capturer pre-initialized");   // [+2834 ms]

info!("🎬 Starting recording");         // [+0 ms]
info!("✅ Recording started");          // [+43 ms] ✓ Fast!
```

If startup exceeds 100ms:
```
error!("⚠️ SLOW START DETECTED: 1.6s from key press to recording started");
error!("💔 We sincerely apologize - you may have lost the first 1.6s of your recording.");
```

## Usage

The logger is automatically initialized in `main.rs`:

```rust
fn main() {
    pulse_desktop_lib::logging::init();
    pulse_desktop_lib::run()
}
```

No additional configuration needed. All log output goes to stdout with automatic delta time tracking.

## Future Enhancements

Potential improvements:
- [ ] Environment variable control for log level (`RUST_LOG=debug`)
- [ ] File output option for persistent logs
- [ ] Structured logging (JSON format) for log aggregation
- [ ] Per-module log level configuration
- [ ] Log rotation for long-running sessions
- [ ] Integration with crash reporting systems

## Migration Summary

- **Total files modified**: 11
- **Total print statements replaced**: ~100+
- **Build status**: ✅ Successful (0 errors, 14 warnings - all unused code)
- **New dependencies**: `log 0.4`, `env_logger 0.11`
- **Lines of code added**: ~40 (logging module)
- **Lines of code modified**: ~200 (print statement replacements)

All critical paths now use appropriate log levels:
- ✅ Recording performance monitoring (with delta times)
- ✅ Error conditions properly logged with context
- ✅ "Hand written apologies" for slow starts preserved (ERROR level)
- ✅ Filesystem watcher operations tracked
- ✅ Pre-initialization milestones logged
