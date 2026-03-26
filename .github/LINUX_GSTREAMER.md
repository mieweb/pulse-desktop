# Linux Screen Capture — Implementation Plan

## Current State

Both stub files exist, returning `Err("not implemented")`:
- `crates/screen-capture/src/linux.rs` — `NativeRecorder` (low-level capture + encode)
- `src-tauri/src/capture/linux.rs` — `ScreenCapturer` (high-level Tauri-facing wrapper)

The macOS path (`ScreenCaptureKit` + `AVAssetWriter`) and Windows path (`Desktop Duplication API` + `Media Foundation`) are the reference implementations to mirror.

---

## Technology Choices

### Screen Capture: PipeWire via XDG Desktop Portal

Use the [`ashpd`](https://crates.io/crates/ashpd) crate to request a ScreenCast stream through the `org.freedesktop.portal.ScreenCast` XDG portal.

**Why:**
- Works on both **Wayland and X11** (portal handles the difference transparently)
- Required by modern distros — direct X11 grabs are actively being phased out
- Matches how OBS, Zoom, and other apps capture on Linux today
- `ashpd` is async and ergonomic; no raw D-Bus needed

**Fallback:** If `DISPLAY` is set and `WAYLAND_DISPLAY` is not, optionally fall back to direct X11 framebuffer grab via `x11rb` without needing portal consent. This is simpler for dev/testing.

### Encoding: GStreamer Pipeline

Use the [`gstreamer`](https://crates.io/crates/gstreamer) + [`gstreamer-app`](https://crates.io/crates/gstreamer-app) crates to build an encode pipeline:

```
appsrc → videoconvert → x264enc → mp4mux → filesink
      ↗ (audio from PipeWire or PulseAudio when mic enabled)
pulsesrc/pipewiresrc → audioconvert → voaacenc → mp4mux
```

**Why GStreamer over FFmpeg:**
- Available on all major Linux distros as a system library
- Native hardware-accelerated paths (VA-API, NVENC) via `vaapih264enc` element
- No subprocess; runs in-process with clean Rust FFI
- Consistent with what the portal delivers (DMA-BUF → GStreamer is well-supported)

---

## Dependency Plan

Add to `crates/screen-capture/Cargo.toml`:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
ashpd = { version = "0.9", features = ["tokio"] }       # XDG Desktop Portal
gstreamer = "0.23"                                        # Pipeline orchestration
gstreamer-app = "0.23"                                    # appsrc/appsink
gstreamer-video = "0.23"                                  # video caps helpers
x11rb = { version = "0.13", optional = true }            # X11 fallback
pipewire = "0.8"                                          # Direct PipeWire stream consumer

[features]
linux-x11-fallback = ["x11rb"]
```

System packages required at build time: `libgstreamer1.0-dev`, `libgstreamer-plugins-base1.0-dev`, `gstreamer1.0-plugins-good`, `gstreamer1.0-plugins-bad`, `libpipewire-0.3-dev`.

---

## Implementation Phases

### Phase 1 — Build System & Dependency Wiring

**Files:** `crates/screen-capture/Cargo.toml`, `crates/screen-capture/build.rs`

1. Add Linux target dependencies above. Update README with these new packages.
2. Extend `build.rs` to run `pkg-config` for `gstreamer-1.0` and `pipewire-0.3`, emitting `cargo:rustc-link-lib` directives.
3. Confirm the workspace compiles without errors (stubs still return `Err`).

---

### Phase 2 — Portal ScreenCast Session

**File:** `crates/screen-capture/src/linux.rs`

Implement `NativeRecorder::new()` to:

```
1. Connect to the D-Bus session bus via ashpd
2. Call ScreenCast portal: CreateSession → SelectSources → Start
3. Receive the PipeWire stream node ID from the portal response
4. Open a pipewire::Context, connect the node
5. Store the stream handle; ready to pull frames
```

Key types:
- `ashpd::desktop::screencast::ScreenCastProxy` — the portal interface
- `ashpd::desktop::screencast::SourceType::Monitor | Window`
- `pipewire::stream::Stream` — frame consumer

The portal Start call returns a `Streams` list; use the first stream's `pipewire_node_id`.

---

### Phase 3 — Encode Pipeline

**File:** `crates/screen-capture/src/linux.rs`

Build the GStreamer pipeline on `NativeRecorder::start()`:

```rust
gstreamer::parse::launch(
    "appsrc name=vsrc is-live=true format=time \
     ! video/x-raw,format=BGRx \
     ! videoconvert ! x264enc tune=zerolatency \
     ! h264parse ! mp4mux name=mux \
     ! filesink location={path}"
)?
```

Push PipeWire frames into `appsrc` on each `process` callback.

For microphone audio (when `mic_enabled`):
```rust
// Append to the pipeline string:
"pulsesrc device={device} ! audioconvert ! voaacenc ! mux."
```

`NativeRecorder::stop()` sends EOS, waits for `GstMessage::Eos`, queries duration from the mux, returns `(output_path, duration_secs)`.

---

### Phase 4 — Region Capture

**File:** `crates/screen-capture/src/linux.rs`

When `RecordingConfig::region` is `Some((x, y, w, h))`, insert a `videocrop` element after `appsrc`:

```
appsrc ! video/x-raw ... ! videocrop top=y left=x right=... bottom=... ! videoconvert ...
```

PipeWire delivers the full monitor frame; cropping is done in the GStreamer pipeline. This matches the macOS pattern where `CaptureRegion` is passed to `RecordingConfig`.

---

### Phase 5 — ScreenCapturer Wrapper

**File:** `src-tauri/src/capture/linux.rs`

Mirror the macOS `ScreenCapturer` struct exactly:

```rust
pub struct ScreenCapturer {
    output_folder: PathBuf,
    is_recording: bool,
    start_time: Option<Instant>,
    recorder: Option<NativeRecorder>,
    mic_enabled: bool,
    audio_device_id: Option<String>,
    pre_initialized: bool,
    prepared_output_path: Option<PathBuf>,
}
```

Implement the three async methods:

| Method | Linux behavior |
|---|---|
| `pre_initialize(region)` | Runs portal session setup + pipeline construction so `start_recording` is near-instant |
| `start_recording(region)` | Calls `recorder.start()`, stores `Instant::now()` |
| `stop_recording()` | Calls `recorder.stop()`, returns `(path, duration)` |

Pre-initialization on Linux involves the portal dialog (user clicks "Share" once per session), so the first call will have visible latency. Subsequent recordings within the same session reuse the open portal session and are fast.

---

### Phase 6 — X11 Fallback (Optional)

**File:** `crates/screen-capture/src/linux.rs` (behind `#[cfg(feature = "linux-x11-fallback")]`)

Detect session type at runtime:

```rust
let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
```

If X11 session and the portal is unavailable, use `x11rb` to:
1. `xcb_shm_get_image` each frame at the requested FPS into a shared memory segment
2. Push frames into the same GStreamer `appsrc` pipeline

This requires no portal consent dialog and is useful for CI/headless environments.

---

## File Change Summary

| File | Change |
|---|---|
| `crates/screen-capture/Cargo.toml` | Add Linux target dependencies |
| `crates/screen-capture/build.rs` | Add pkg-config for gstreamer, pipewire |
| `crates/screen-capture/src/linux.rs` | Full `NativeRecorder` implementation |
| `src-tauri/src/capture/linux.rs` | Full `ScreenCapturer` wrapper |
| `src-tauri/Cargo.toml` | No changes needed (already uses path dep) |

No changes to `commands.rs`, `state.rs`, or any frontend code — the Linux implementation slots into the existing platform abstraction.

---

## Testing Approach

1. **Unit:** Mock the GStreamer pipeline with a `fakesink`; assert `start()`/`stop()` transitions work.
2. **Integration:** Record a 2-second clip, verify the output file exists, is non-zero bytes, and `ffprobe` reports valid H.264/AAC streams.
3. **Manual:** Verify the portal "Select what to share" dialog appears on Wayland; verify no dialog on X11 fallback.
4. **Hotkey:** Hold `Ctrl+Shift+R`, confirm recording starts and stops, file appears in `~/Videos/PushToHold/`.

---

## Known Constraints

- **Portal consent dialog** — On Wayland, the user must click "Share" in the portal dialog the first time (and possibly each session, depending on the compositor). This is unavoidable; it is the same behavior as `obs`, `kooha`, and `gnome-screencast`.
- **GStreamer plugins** — `x264enc` and `voaacenc` require `gstreamer1.0-plugins-ugly` and `gstreamer1.0-plugins-bad` respectively; these may not be installed by default on all distros. The error message should guide the user.
- **Hardware acceleration** — VA-API (`vaapih264enc`) can be substituted for `x264enc` at runtime by checking for the element's availability with `gstreamer::ElementFactory::find("vaapih264enc")`.
