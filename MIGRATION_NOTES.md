# 🚧 Migration to Native Screen Capture

**Date**: October 12, 2025  
**Status**: Foundation complete, implementation pending

## What Changed?

We discovered that using `screenshots` crate + FFmpeg transcoding was causing:
- Retina scaling bugs (captured 2940×1912 but encoded as 1470×956)
- High memory usage (buffering frames as PNGs)
- CPU-intensive transcoding (PNG → RGB → YUV → H.264)
- Large file sizes (software encoder)

## New Approach

Created a new crate: **`screen-capture`** that uses native OS APIs:

- **macOS**: ScreenCaptureKit + AVAssetWriter + VideoToolbox
- **Windows**: Desktop Duplication API + Media Foundation

This provides:
- ✅ Direct MP4 encoding (no transcoding)
- ✅ Hardware acceleration
- ✅ Proper Retina/HiDPI support
- ✅ Low memory (streaming)
- ✅ Smaller files

## Project Structure

```
pulse-desktop/
├── crates/
│   └── screen-capture/        ← NEW: Native capture crate
│       ├── src/
│       │   ├── lib.rs         ← Cross-platform API
│       │   ├── macos.rs       ← ScreenCaptureKit (stub)
│       │   └── windows.rs     ← Desktop Duplication (stub)
│       ├── examples/
│       ├── IMPLEMENTATION_PLAN.md
│       └── SUMMARY.md
├── src-tauri/
│   └── src/
│       ├── capture/macos.rs   ← Will use screen-capture crate
│       └── encoding/mod.rs    ← Can be deleted once migration complete
└── .github/
    ├── MP4_ENCODING_*.md      ← FFmpeg approach (deprecated)
    └── DEVELOPMENT_PROGRESS.md ← Updated with new direction
```

## Current Status

### ✅ Complete
- Crate structure created
- Public API designed
- Platform detection working
- Compiles successfully
- Documentation written

### 🚧 In Progress  
- macOS ScreenCaptureKit implementation (Objective-C bridge needed)
- Windows Desktop Duplication implementation

### ⏳ Next Steps
1. Implement Objective-C bridge for ScreenCaptureKit
2. Test basic capture on macOS
3. Integrate into pulse-desktop
4. Remove FFmpeg dependencies
5. Implement Windows version

## Documentation

- [Implementation Plan](crates/screen-capture/IMPLEMENTATION_PLAN.md)
- [Summary](crates/screen-capture/SUMMARY.md)
- [Development Progress](.github/DEVELOPMENT_PROGRESS.md)

## Why This Matters

This is the **correct architecture** for a production screen recorder. The FFmpeg approach was a proof-of-concept that revealed the Retina scaling issue. Native APIs are the proper solution.

---

**Note**: The current FFmpeg-based code still works (with scrambled video due to Retina bug). We're not removing it until the native implementation is ready.
