# Pulse Desktop

Check out [pulse](https://github.com/mieweb/pulse) to get pulse and this repo together.

**Minimal push-to-hold screen recorder for macOS and Windows**

Push-to-hold screen capture with the simplicity of a walkie-talkie. Press `Cmd+Shift+R` (macOS) or `Ctrl+Shift+R` (Windows), hold to record, release to save. Videos are automatically saved to your Movies folder.

> **🚧 Architecture Update**: Migrating from FFmpeg-based transcoding to native OS APIs (ScreenCaptureKit + Desktop Duplication) for better performance and Retina support. See [MIGRATION_NOTES.md](MIGRATION_NOTES.md) for details.

## Features

- ✅ **Push-to-hold recording** - Press and hold to record, release to save
- ✅ **Global hotkey** - Works from any application (`Cmd+Shift+R` / `Ctrl+Shift+R`)
- ✅ **Full screen capture** - Records entire display at 30 FPS
- 🚧 **MP4 video output** - Migrating to native H.264 encoding (hardware accelerated)
- ✅ **Sequential file naming** - `recording-1.mp4`, `recording-2.mp4`, etc.
- ✅ **Automatic folder creation** - Saves to `~/Movies/PushToHold` (macOS) or `~/Videos/PushToHold` (Windows)
- ⏳ Microphone audio toggle (coming soon)
- ⏳ Region selection with aspect ratio presets (coming soon)

## System Requirements

### macOS
- macOS 11.0 or later
- Screen Recording permission (prompted on first use)
- **FFmpeg** (temporary, until native implementation complete)
  ```bash
  brew install ffmpeg
  ```

### Windows
- Windows 10 or later
- **FFmpeg** (temporary, until native implementation complete)
  - Download from [ffmpeg.org](https://ffmpeg.org/download.html)
  - Add to PATH

## Quick Start

### Prerequisites
- **Deno** - JavaScript/TypeScript runtime ([install](https://deno.com/))
- **Rust** - For Tauri backend ([install](https://www.rust-lang.org/tools/install))
- **Xcode Command Line Tools** (macOS only)
  ```bash
  xcode-select --install
  ```

### Running the App

```bash
# Clone and navigate to the project
cd pulse-desktop

# Run in development mode
deno task tauri dev
```

This will:
1. Start the Vite dev server (React frontend)
2. Compile and run the Rust/Tauri backend
3. Open the desktop application window

### Other Commands

```bash
# Build for production
deno task tauri build

# Run frontend only (UI development)
deno task dev

# Run tests
deno task test
```

## Using Pulse Desktop

1. **Launch the app** - Run `deno task tauri dev`
2. **Create/select a project** - Choose a project name for organizing your recordings
3. **Record** - Press and hold `Cmd+Shift+R` (macOS) or `Ctrl+Shift+R` (Windows)
4. **Release to save** - Your recording is automatically saved
5. **Manage clips** - View your timeline, drag to reorder clips, edit labels, or delete

### Timeline Features
- **Drag-drop reordering** - Click and hold any clip, drag to new position (blue pulsing line shows where it will drop)
- **Keyboard navigation** - Use arrow keys to navigate, Cmd+Arrow to reorder
- **Edit labels** - Click on any clip filename to rename
- **Undo/Redo** - Cmd+Z / Cmd+Shift+Z for timeline changes

## Development

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Running for developers

```
npm install
npm run tauri dev
```
