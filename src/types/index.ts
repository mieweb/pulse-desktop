// Core types for Pulse Desktop

export type CaptureMode = 'full' | 'region';
export type AspectRatio = '16:9' | '9:16' | 'none';
export type RecordingStatus = 'idle' | 'recording' | 'saving' | 'error';

export interface CaptureRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OutputResolution {
  width: number;
  height: number;
}

export interface RecordingState {
  status: RecordingStatus;
  clipCount: number;
  currentClipPath?: string;
  error?: string;
}

export interface AppSettings {
  outputFolder: string;
  captureMode: CaptureMode;
  aspectRatio: AspectRatio;
  scaleToPreset: boolean;
  micEnabled: boolean;
  captureRegion?: CaptureRegion;
}

// Events from Rust backend
export interface ClipSavedEvent {
  path: string;
  durationMs: number;
}

export interface ErrorEvent {
  code: string;
  message: string;
}

// Preset resolutions for aspect ratios
export const ASPECT_PRESETS: Record<AspectRatio, OutputResolution[]> = {
  '16:9': [
    { width: 1920, height: 1080 },
    { width: 2560, height: 1440 },
    { width: 3840, height: 2160 },
  ],
  '9:16': [
    { width: 1080, height: 1920 },
    { width: 1440, height: 2560 },
    { width: 2160, height: 3840 },
  ],
  'none': [],
};
