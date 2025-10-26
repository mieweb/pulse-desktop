//
//  SCRecorder.h
//  ScreenCaptureKit wrapper for Rust
//
//  Provides C API for ScreenCaptureKit + AVAssetWriter
//

#ifndef SCRecorder_h
#define SCRecorder_h

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque recorder handle
typedef struct SCRecorder SCRecorder;

// Event callback
typedef void (*SCRecorderCallback)(int32_t event, void* user_data);

// Events
#define SC_EVENT_STARTED 0
#define SC_EVENT_STOPPED 1
#define SC_EVENT_ERROR   2
#define SC_EVENT_FRAME   3

// Create a new recorder
// Returns NULL on failure
// audio_device_id: optional device unique ID (NULL for auto-select)
SCRecorder* sc_recorder_create(
    const char* output_path,
    uint32_t width,
    uint32_t height,
    uint32_t fps,
    uint32_t quality,
    uint32_t display_id,
    bool capture_audio,
    const char* audio_device_id
);

// Start recording
// Returns 0 on success, non-zero on error
int32_t sc_recorder_start(SCRecorder* recorder);

// Stop recording
// Returns 0 on success, non-zero on error
int32_t sc_recorder_stop(SCRecorder* recorder);

// Get recording duration in seconds
double sc_recorder_duration(SCRecorder* recorder);

// Free the recorder
void sc_recorder_free(SCRecorder* recorder);

// Set callback for events
void sc_recorder_set_callback(
    SCRecorder* recorder,
    SCRecorderCallback callback,
    void* user_data
);

// Get last error message (NULL if no error)
const char* sc_recorder_last_error(SCRecorder* recorder);

// Rust logging bridge functions
// These allow Objective-C code to log through Rust's log system
void rust_log_info(const char* msg);
void rust_log_debug(const char* msg);
void rust_log_warn(const char* msg);
void rust_log_error(const char* msg);

#ifdef __cplusplus
}
#endif

#endif /* SCRecorder_h */
