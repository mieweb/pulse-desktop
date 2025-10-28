use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClipSavedEvent {
    pub path: String,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorEvent {
    pub code: String,
    pub message: String,
}

/// Emit recording status to frontend
pub fn emit_status(app: &AppHandle, status: &str) -> Result<(), String> {
    app.emit("recording-status", status)
        .map_err(|e| format!("Failed to emit status: {}", e))
}

/// Emit pre-init status to frontend
pub fn emit_pre_init_status(app: &AppHandle, status: &str) -> Result<(), String> {
    app.emit("pre-init-status-changed", status)
        .map_err(|e| format!("Failed to emit pre-init status: {}", e))
}

/// Emit clip saved event to frontend
pub fn emit_clip_saved(app: &AppHandle, event: ClipSavedEvent) -> Result<(), String> {
    app.emit("clip-saved", event)
        .map_err(|e| format!("Failed to emit clip-saved: {}", e))
}

/// Emit error event to frontend
pub fn emit_error(app: &AppHandle, code: &str, message: &str) -> Result<(), String> {
    app.emit(
        "recording-error",
        ErrorEvent {
            code: code.to_string(),
            message: message.to_string(),
        },
    )
    .map_err(|e| format!("Failed to emit error: {}", e))
}

/// Emit project required event to frontend (when recording starts without a project)
pub fn emit_project_required(app: &AppHandle) -> Result<(), String> {
    app.emit("project-required", ())
        .map_err(|e| format!("Failed to emit project-required: {}", e))
}
