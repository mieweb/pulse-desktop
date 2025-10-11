# Pulse Desktop - Development Progress

## ✅ Phase 1: Project Foundation (COMPLETED)

### Project Structure Created
```
pulse-desktop/
├── src/
│   ├── types/index.ts           ✅ Core TypeScript types
│   ├── components/               ✅ UI components
│   │   ├── StatusChip.tsx       ✅ Recording status indicator
│   │   └── SettingsPanel.tsx    ✅ Settings controls
│   ├── hooks/                   ✅ React hooks
│   │   ├── useRecording.ts      ✅ Recording state management
│   │   └── useSettings.ts       ✅ App settings management
│   └── App.tsx                   ✅ Main application component
└── src-tauri/
    └── src/
        ├── lib.rs                ✅ Tauri entry point
        ├── commands.rs           ✅ Tauri commands
        ├── state.rs              ✅ App state management
        ├── events.rs             ✅ Event emissions
        ├── capture/              ✅ Screen capture module
        │   ├── mod.rs
        │   ├── macos.rs          ✅ macOS ScreenCaptureKit (stub)
        │   └── windows.rs        ✅ Windows Desktop Duplication (stub)
        └── hotkey/               ✅ Global hotkey module
            ├── mod.rs
            ├── macos.rs          ✅ CGEventTap (stub)
            └── windows.rs        ✅ SetWindowsHookEx (stub)
```

### Implemented Features

#### Frontend (React + TypeScript)
- [x] Basic UI layout with header and status chip
- [x] Settings panel component
- [x] Type definitions for all core data structures
- [x] Custom hooks for recording and settings state
- [x] Event listeners for backend communication
- [x] Accessibility (ARIA labels) implemented
- [x] Error and success message display

#### Backend (Tauri + Rust)
- [x] Project compiles successfully on macOS
- [x] Basic Tauri commands:
  - `set_output_folder` - Configure save location
  - `get_output_folder` - Retrieve current folder
  - `set_mic_enabled` - Toggle microphone
  - `authorize_capture` - Request permissions
  - `init_hotkey` - Initialize global hotkey (stub)
  - `start_recording` - Manual recording start (stub)
  - `stop_recording` - Manual recording stop (stub)
- [x] Platform-specific module structure (macOS/Windows)
- [x] Screen capture module skeleton
- [x] Hotkey module skeleton
- [x] Event emission system

### Current Status
✅ **App is running!** 
- Dev server: http://localhost:1420/
- Compilation: Clean (13 warnings, 0 errors)
- Platform: macOS (primary development)

---

## 🚧 Phase 2: Core Recording Implementation (IN PROGRESS)

### ✅ Completed: Hotkey Registration (Priority 1)

**Status**: ✅ Implemented with tauri-plugin-global-shortcut

**Implementation Details:**
- ✅ Added `tauri-plugin-global-shortcut` v2.3.0 to dependencies
- ✅ Registered global shortcut `Cmd+Shift+R` (macOS) / `Ctrl+Shift+R` (Windows)
- ✅ Implemented press/release detection (not just click)
- ✅ Added atomic boolean to prevent race conditions
- ✅ Wired up to event emission system
- ✅ Status updates: idle → recording → idle (removed "saving" state to fix race condition)
- ✅ **Race condition fixed**: Immediate status transition prevents stale events during rapid key presses
- ✅ Debug logging added for event tracking

**Files Modified:**
- `src-tauri/Cargo.toml` - Added global-shortcut plugin
- `src-tauri/src/lib.rs` - Initialized plugin and setup hook
- `src-tauri/src/commands.rs` - Implemented `setup_global_shortcut()`
- `src-tauri/src/state.rs` - Added recording state tracking

**How It Works:**
```rust
// Register shortcut on app startup
app.global_shortcut().on_shortcut(shortcut, |app, _, event| {
    match event.state {
        ShortcutState::Pressed => {
            // Start recording
            emit_status(app, "recording");
        }
        ShortcutState::Released => {
            // Stop recording
            emit_status(app, "saving");
            // Save file
            emit_clip_saved(app, ClipSavedEvent { ... });
        }
    }
});
```

**Testing Status:**
- ⏳ Needs testing: Hotkey across different foreground applications
- ⏳ Needs testing: Debounce verification (auto-repeat prevention)
- ⏳ Needs testing: Multiple rapid press/release cycles

**Why tauri-plugin-global-shortcut?**
1. **Official Tauri plugin** - Best integration with Tauri v2
2. **Cross-platform** - Works on macOS, Windows, Linux
3. **Press/Release events** - Essential for push-to-hold recording
4. **Maintained** - Active development and support
5. **Simple API** - Clean integration compared to rdev or global-hotkey

**Alternatives Considered:**
- ❌ `rdev` - Lower-level, requires more boilerplate, OS-specific code
- ❌ `global-hotkey` - Good alternative but not Tauri-specific
- ✅ `tauri-plugin-global-shortcut` - **BEST CHOICE** for Tauri apps

---

### Next Steps - Development Checklist

#### 1. ✅ Hotkey Registration (Priority 1) - COMPLETED
**Goal**: Make global hotkey (Cmd+Shift+R) functional

**Implementation:**
- [x] Added `tauri-plugin-global-shortcut` to Cargo.toml
- [x] Implemented global shortcut registration in setup hook
- [x] Added press/release event handlers  
- [x] Atomic boolean prevents race conditions (debounce built-in)
- [x] Events emitted to frontend (recording, saving, idle, clipSaved)

**Files modified:**
- `src-tauri/Cargo.toml` - Plugin dependency
- `src-tauri/src/lib.rs` - Plugin init and setup
- `src-tauri/src/commands.rs` - Shortcut registration and handling
- `src-tauri/src/state.rs` - Recording state tracking

**Testing needed:**
- [ ] Test with different foreground applications (Chrome, VS Code, Finder, etc.)
- [ ] Verify no multiple recordings from key repeat
- [ ] Test rapid press/release cycles
- [ ] Verify status updates in UI

**Current behavior:**
- Press Cmd+Shift+R: Console logs "🎬 Starting recording...", emits "recording" status
- Release Cmd+Shift+R: Console logs "⏹️ Stopping recording...", emits "saving" then "idle" status, emits clipSaved event

---

#### 2. 🟡 Basic Screen Capture (Priority 2)
**Goal**: Capture full screen on macOS

**macOS Implementation:**
- [ ] Add screen capture dependencies (screencapturekit-rs or similar)
- [ ] Implement ScreenCaptureKit authorization
- [ ] Capture full screen to frames
- [ ] Basic encoding to MP4 (no scaling yet)
- [ ] Save to output folder

**Files to modify:**
- `src-tauri/Cargo.toml` - Add capture dependencies
- `src-tauri/src/capture/macos.rs` - Implement capture
- `src-tauri/src/commands.rs` - Wire capture to start/stop

**Acceptance criteria:**
- [ ] Permission prompt appears on first run
- [ ] Full screen captured successfully
- [ ] Output file is playable MP4
- [ ] Recording duration ±150ms of hold time

---

#### 3. 🟡 File Management (Priority 2)
**Goal**: Sequential file naming and folder creation

**Implementation:**
- [ ] Implement sequential numbering (recording-1, recording-2...)
- [ ] Auto-create output folder if missing
- [ ] Default folders:
  - macOS: `~/Movies/PushToHold`
  - Windows: `~/Videos/PushToHold`
- [ ] Track clip count in state
- [ ] Emit `clipSaved` event with path and duration

**Files to modify:**
- `src-tauri/src/state.rs` - Add clip counter
- `src-tauri/src/commands.rs` - File naming logic
- `src-tauri/src/events.rs` - Use clipSaved event

**Acceptance criteria:**
- [ ] Files named sequentially
- [ ] No overwrites
- [ ] Clip count updates in UI
- [ ] Success message shows file path

---

#### 4. 🟢 Microphone Audio (Priority 3)
**Goal**: Toggle mic recording on/off

**Implementation:**
- [ ] Add audio capture dependency
- [ ] Capture mic input when enabled
- [ ] Mix audio with video stream
- [ ] Respect mic toggle setting

**Files to modify:**
- `src-tauri/Cargo.toml` - Add audio dependencies
- `src-tauri/src/capture/macos.rs` - Mic capture logic

**Acceptance criteria:**
- [ ] Recording with mic ON has audio
- [ ] Recording with mic OFF has no audio
- [ ] Audio synced with video

---

#### 5. ⚪ Region Selection (Priority 4)
**Goal**: User-defined capture region with aspect ratio snapping

**Frontend:**
- [ ] Create RegionOverlay component
- [ ] Draggable/resizable region selector
- [ ] Aspect ratio constraint (16:9, 9:16)
- [ ] Show computed output resolution
- [ ] Cancel/Confirm actions

**Backend:**
- [ ] Store region coordinates in state
- [ ] Apply region to capture API
- [ ] Scale to preset resolution (if enabled)

**Files to create:**
- `src/components/RegionOverlay.tsx`
- `src/components/RegionOverlay.css`

**Files to modify:**
- `src-tauri/src/capture/macos.rs` - Region capture
- `src-tauri/src/commands.rs` - Region selection commands

**Acceptance criteria:**
- [ ] Overlay appears on button click
- [ ] Smooth drag and resize
- [ ] Snaps to selected aspect ratio
- [ ] Shows output resolution when scale enabled
- [ ] Captured region matches selection

---

#### 6. ⚪ Aspect Ratio & Scaling (Priority 4)
**Goal**: Scale captured content to preset resolutions

**Implementation:**
- [ ] Add video scaling during encoding
- [ ] Calculate nearest preset resolution
- [ ] Display computed resolution in UI
- [ ] Apply scaling transform

**Presets:**
- 16:9: 1920×1080, 2560×1440, 3840×2160
- 9:16: 1080×1920, 1440×2560, 2160×3840

**Acceptance criteria:**
- [ ] Scaling ON: Output matches preset
- [ ] Scaling OFF: Output matches capture size
- [ ] Quality maintained during scaling

---

#### 7. ⚪ Windows Support (Priority 5)
**Goal**: Port all features to Windows

**Implementation:**
- [ ] Desktop Duplication API for capture
- [ ] SetWindowsHookEx for Ctrl+Shift+R
- [ ] Media Foundation for encoding
- [ ] Test all features on Windows

**Files to modify:**
- `src-tauri/src/capture/windows.rs`
- `src-tauri/src/hotkey/windows.rs`

**Acceptance criteria:**
- [ ] All macOS features work on Windows
- [ ] Hotkey is Ctrl+Shift+R
- [ ] Output to ~/Videos/PushToHold

---

## 📋 Testing Strategy

### Smoke Tests (per feature)
1. **Hotkey**: Hold Cmd+Shift+R with different apps in foreground
2. **Capture**: Full screen, verify playback and duration
3. **Files**: Check sequential naming, no overwrites
4. **Mic**: Toggle on/off, verify audio presence
5. **Region**: Drag region, check snapping to aspect
6. **Scaling**: ON/OFF modes, verify output resolution
7. **Cross-platform**: Repeat 1-6 on Windows

### Integration Tests
- [ ] Full workflow: Launch → Set folder → Enable mic → Region → Record → Verify file
- [ ] Error handling: No permission, disk full, invalid region
- [ ] Edge cases: Rapid hotkey presses, region too small

---

## 📖 Development Guide

### Building and Running

```bash
# Install dependencies (if not already)
npm install  # or deno install

# Run development mode
npm run tauri dev

# Build for production
npm run tauri build

# Check Rust code
cd src-tauri && cargo check
```

### Adding a New Tauri Command

1. Define command in `src-tauri/src/commands.rs`:
```rust
#[tauri::command]
pub fn my_command(param: String) -> Result<String, String> {
    Ok(format!("Received: {}", param))
}
```

2. Register in `src-tauri/src/lib.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    commands::my_command,
])
```

3. Call from React:
```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke<string>('my_command', { param: 'test' });
```

### Platform-Specific Code

```rust
#[cfg(target_os = "macos")]
{
    // macOS-specific code
}

#[cfg(target_os = "windows")]
{
    // Windows-specific code
}
```

---

## 🎯 Current Focus

**Working on**: Phase 2, Step 1 - Hotkey Registration

**Next session**:
1. Research best Rust crate for global hotkeys (rdev vs global-hotkey vs tauri-plugin-global-shortcut)
2. Implement macOS Cmd+Shift+R hotkey
3. Test hotkey across different applications
4. Move to screen capture implementation

---

## 📝 Notes

- Project uses Deno for package management (not npm/yarn)
- Tauri plugins: dialog, opener (already installed)
- Target: macOS first, Windows second
- V0 scope: No system audio, no multi-monitor UI, no timeline editing
- Code philosophy: DRY, KISS, clear folder structure

---

**Last updated**: October 10, 2025  
**Status**: ✅ Foundation complete, 🚧 Core features in progress
