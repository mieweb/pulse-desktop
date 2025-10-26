# Pulse Desktop - TODO

## 🎯 Future Feature TODOs

### Global Hotkey System (High Priority)

**Status:** Placeholder implementation exists  
**Related Files:**
- `src-tauri/src/hotkey/mod.rs`
- `src-tauri/src/hotkey/macos.rs`
- `src-tauri/src/hotkey/windows.rs`

**Tasks:**
- [ ] **Implement macOS hotkey registration** (`src-tauri/src/hotkey/macos.rs`)
  - Implement `MacOSHotkeyManager::new()` to initialize CGEventTap
  - Implement `register()` method to create event tap for Cmd+Shift+R
  - Filter for Command+Shift+R key combination
  - Call callback(true) on keyDown, callback(false) on keyUp
  - Add debounce logic to prevent auto-repeat events
  - Request accessibility permissions if needed

- [ ] **Implement Windows hotkey registration** (`src-tauri/src/hotkey/windows.rs`)
  - Implement using SetWindowsHookEx for Ctrl+Shift+R
  - Filter for Control+Shift+R key combination
  - Mirror macOS callback behavior
  - Add debounce logic

- [ ] **Integrate with recording system**
  - Replace DebugControls UI buttons with global hotkey triggers
  - Wire up `HotkeyManager` trait in main application state
  - Handle hotkey registration on app startup
  - Handle cleanup on app shutdown

- [ ] **Testing**
  - Test across different foreground applications
  - Verify debouncing works correctly
  - Test permission flows on macOS
  - Test with different keyboard layouts

---

### Screen Capture Enhancements

**Related Files:**
- `src-tauri/src/capture/macos.rs`
- `crates/screen-capture/src/macos/bridge.rs`

**Tasks:**

#### Permission Management
- [ ] **Explicit permission checking** (`src-tauri/src/capture/macos.rs:78`)
  - Implement `ScreenCapturer::request_permission()` to explicitly check screen recording permission
  - Add UI indicator for permission status
  - Provide better error messages when permission is denied
  - Currently: ScreenCaptureKit prompts automatically on first use

#### File Management
- [ ] **Recording path tracking** (`src-tauri/src/capture/macos.rs:175`)
  - Implement `get_last_created_path()` usage for clip management
  - Add UI to show most recently created recording
  - Enable quick access to last recording (reveal in Finder/Explorer)
  - Enable deletion or renaming of clips

#### Real-time Recording Feedback
- [ ] **Duration display** (`src-tauri/src/capture/macos.rs:202`)
  - Use `duration()` method to show live recording timer in UI
  - Update UI every second during recording
  - Display elapsed time in StatusChip component
  - Show recording indicator with animated timer

#### Recording State Access
- [ ] **Recording state queries** (`src-tauri/src/capture/macos.rs:196`)
  - Implement `is_recording()` accessor for external state checks
  - Add Tauri command to query recording state from frontend
  - Use for conditional UI rendering
  - Currently: State managed via atomic bools in `commands.rs`

---

### Event Callback System

**Status:** FFI interface defined but not implemented  
**Related Files:**
- `crates/screen-capture/src/macos/bridge.rs`

**Tasks:**
- [ ] **Implement event callback system** (`crates/screen-capture/src/macos/bridge.rs`)
  - Implement `sc_recorder_set_callback()` FFI function
  - Define event types: `SC_EVENT_STARTED`, `SC_EVENT_STOPPED`, `SC_EVENT_ERROR`, `SC_EVENT_FRAME`
  - Create Objective-C callback mechanism in `SCRecorder.m`
  - Bridge events from Objective-C to Rust via FFI
  - Emit Tauri events for frontend consumption

- [ ] **Frontend event handlers**
  - Listen for recording events in React
  - Update UI based on real-time events
  - Show error notifications from recorder
  - Display frame-by-frame progress if needed

---

### Configuration Management

**Related Files:**
- `crates/screen-capture/src/lib.rs`
- `crates/screen-capture/src/macos/mod.rs`

**Tasks:**
- [ ] **Use stored `RecordingConfig`** 
  - `crates/screen-capture/src/lib.rs:144` - Query config after recorder creation
  - `crates/screen-capture/src/macos/mod.rs:67` - Reconfigure recording settings dynamically
  - Enable runtime config changes (resolution, codec, bitrate)
  - Add UI for advanced recording settings
  - Persist user preferences

---

### Filesystem Watcher Enhancements

**Related Files:**
- `src-tauri/src/fs_watcher.rs`

**Tasks:**
- [ ] **Watcher status utilities** (`src-tauri/src/fs_watcher.rs:28`)
  - Use `is_enabled()` for debugging watcher state
  - Add logging for watcher enable/disable events
  - Expose watcher status in UI (dev mode)
  - Add diagnostics for troubleshooting filesystem events

---

### Application State

**Related Files:**
- `src-tauri/src/state.rs`

**Tasks:**
- [ ] **Recording state field usage** (`src-tauri/src/state.rs:18`)
  - Determine if `is_recording` in `AppState` should be removed
  - Currently duplicates state from atomic bools in `commands.rs`
  - Either: Remove redundant field, or consolidate to single source of truth
  - Update architecture documentation to clarify state management

---

## 📋 Code Quality TODOs

### Dead Code Review
After V0 feature completion, review all `#[allow(dead_code)]` attributes to determine if:
1. The code should be implemented and used
2. The code should be removed entirely
3. The code should remain as a future extension point

### Testing Coverage
- [ ] Add unit tests for recording state machine
- [ ] Add integration tests for hotkey registration
- [ ] Add tests for file management utilities
- [ ] Add tests for event callback system

### Documentation
- [ ] Document hotkey implementation in ARCHITECTURE.md
- [ ] Add API documentation for event callback system
- [ ] Document permission flows for macOS and Windows
- [ ] Create troubleshooting guide for common issues

---

## 🔍 Technical Debt

### macOS Deployment Target Mismatch
- **Issue:** `tauri.conf.json` specifies macOS 10.13, but ScreenCaptureKit requires macOS 12.3+
- **Location:** `src-tauri/tauri.conf.json`
- **Tasks:**
  - [ ] Update deployment target to 12.3 in Tauri config
  - [ ] Document minimum macOS version in README
  - [ ] Add runtime version check with helpful error message
  - [ ] Update CI/CD to test on macOS 12.3+

### Audio Device Enumeration
- **Issue:** Simplified to default device only (to avoid linker errors)
- **Location:** `crates/screen-capture/src/macos/SCRecorder.m`
- **Tasks:**
  - [ ] Investigate proper solution for `___isPlatformVersionAtLeast` linker error
  - [ ] Implement full audio device enumeration if needed
  - [ ] Add UI for audio device selection
  - [ ] Test with multiple audio devices

---

## 📝 Notes

- All items marked "reserved for future" are candidates for removal if not implemented by V1.0
- Priority should be given to hotkey implementation as it's core to the app's UX
- Event callback system would enable better error handling and UI feedback
- Configuration management would enable power user features
