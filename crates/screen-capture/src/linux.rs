use crate::RecordingConfig;
use log::warn;
use std::path::PathBuf;

pub struct NativeRecorder {
    _config: RecordingConfig,
}

impl NativeRecorder {
    pub fn new(config: &RecordingConfig) -> Result<Self, String> {
        warn!("🚧 Linux NativeRecorder requested, but Linux screen capture is not implemented yet");

        Ok(Self {
            _config: config.clone(),
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        Err("Linux screen capture is not implemented yet".to_string())
    }

    pub fn stop(&mut self) -> Result<(PathBuf, f64), String> {
        Err("Linux screen capture is not implemented yet".to_string())
    }

    pub fn duration(&self) -> f64 {
        0.0
    }
}
