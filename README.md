# Pulse Desktop

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

## Development

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Running for developers

```
npm install
npm run tauri dev
```
