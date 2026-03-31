use std::path::PathBuf;
use log::warn;

pub struct ScreenCapturer {
    #[allow(dead_code)]
    output_folder: PathBuf,
    #[allow(dead_code)]
    mic_enabled: bool,
    #[allow(dead_code)]
    audio_device_id: Option<String>,
}

impl ScreenCapturer {
    pub fn new(output_folder: PathBuf, mic_enabled: bool, audio_device_id: Option<String>) -> Self {
        Self {
            output_folder,
            mic_enabled,
            audio_device_id,
        }
    }

    pub async fn pre_initialize(&mut self, _region: Option<(u32, u32, u32, u32)>) -> Result<(), String> {
        warn!("🚧 Linux ScreenCapturer pre_initialize called, but Linux capture is not implemented yet");
        Ok(())
    }

    pub async fn start_recording(&mut self, _region: Option<(u32, u32, u32, u32)>) -> Result<(), String> {
        Err("Linux screen capture is not implemented yet".to_string())
    }

    pub async fn stop_recording(&mut self) -> Result<(PathBuf, f64), String> {
        Err("Linux screen capture is not implemented yet".to_string())
    }
}
