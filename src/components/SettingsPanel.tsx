import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';
import type { AppSettings, CaptureMode, AspectRatio, AudioDevice } from '../types';
import './SettingsPanel.css';

interface SettingsPanelProps {
  settings: AppSettings;
  onSettingsChange: (settings: Partial<AppSettings>) => void;
  onCaptureModeChange?: (mode: CaptureMode) => void;
}

export function SettingsPanel({
  settings,
  onSettingsChange,
  onCaptureModeChange,
}: SettingsPanelProps) {
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
  const [loadingDevices, setLoadingDevices] = useState(true);

  useEffect(() => {
    // Load audio devices on mount
    invoke<AudioDevice[]>('get_audio_devices')
      .then((devices) => {
        setAudioDevices(devices);
        
        // If no device is selected, automatically select the preferred one
        if (!settings.selectedAudioDevice && devices.length > 0) {
          const preferredDevice = devices.find(d => d.is_builtin) || devices.find(d => d.is_default) || devices[0];
          if (preferredDevice) {
            onSettingsChange({ selectedAudioDevice: preferredDevice.id });
          }
        }
      })
      .catch((err) => {
        console.error('Failed to load audio devices:', err);
      })
      .finally(() => {
        setLoadingDevices(false);
      });
  }, []);

  const handleCaptureModeChange = (mode: CaptureMode) => {
    onSettingsChange({ captureMode: mode });
    if (onCaptureModeChange) {
      onCaptureModeChange(mode);
    }
  };

  const handleAspectChange = (aspect: AspectRatio) => {
    onSettingsChange({ aspectRatio: aspect });
  };

  const handleScaleToggle = () => {
    onSettingsChange({ scaleToPreset: !settings.scaleToPreset });
  };

  const handleMicToggle = () => {
    const newValue = !settings.micEnabled;
    onSettingsChange({ micEnabled: newValue });
    invoke('set_mic_enabled', { enabled: newValue });
  };

  const handleAudioDeviceChange = (deviceId: string) => {
    onSettingsChange({ selectedAudioDevice: deviceId });
    // Update backend with selected device
    invoke('set_audio_device', { deviceId })
      .catch((err) => console.error('Failed to set audio device:', err));
  };

  const getAudioDeviceLabel = (device: AudioDevice) => {
    let label = device.name;
    if (device.is_builtin) {
      label += ' (Built-in)';
    } else if (device.is_default) {
      label += ' (System Default)';
    }
    return label;
  };

  return (
    <div className="settings-panel">
      {/* Status bar: Output location and clips counter */}

      {/* Capture settings */}
      <div className="capture-settings">
        <div className="setting-group">
          <label className="setting-label">Capture Mode</label>
          <div className="button-group" role="group" aria-label="Capture mode">
            <button
              onClick={() => handleCaptureModeChange('full')}
              className={`capture-button ${settings.captureMode === 'full' ? 'active' : ''}`}
              aria-pressed={settings.captureMode === 'full'}
              title="Full Screen"
            >
              🖥️ Full Screen
            </button>
            <button
              onClick={() => handleCaptureModeChange('region')}
              className={`capture-button ${settings.captureMode === 'region' ? 'active' : ''}`}
              aria-pressed={settings.captureMode === 'region'}
              title="Region"
            >
              ✂️ Region
            </button>
          </div>
        </div>
        
        <div className="setting-group">
          <label className="setting-label">Aspect Ratio</label>
          <div className="button-group" role="group" aria-label="Aspect ratio">
            <button
              onClick={() => handleAspectChange('16:9')}
              className={`aspect-button ${settings.aspectRatio === '16:9' ? 'active' : ''}`}
              aria-pressed={settings.aspectRatio === '16:9'}
            >
              16:9
            </button>
            <button
              onClick={() => handleAspectChange('9:16')}
              className={`aspect-button ${settings.aspectRatio === '9:16' ? 'active' : ''}`}
              aria-pressed={settings.aspectRatio === '9:16'}
            >
              9:16
            </button>
            <button
              onClick={() => handleAspectChange('none')}
              className={`aspect-button ${settings.aspectRatio === 'none' ? 'active' : ''}`}
              aria-pressed={settings.aspectRatio === 'none'}
            >
              Free
            </button>
          </div>
        </div>

        <div className="setting-group checkboxes">
          <div className="toggle-option">
            <input
              type="checkbox"
              id="scale-toggle"
              checked={settings.scaleToPreset}
              onChange={handleScaleToggle}
              className="toggle-input"
              aria-label="Scale to preset resolution"
            />
            <label htmlFor="scale-toggle" className="toggle-label">
              Scale to preset
            </label>
          </div>
          <div className="toggle-option">
            <input
              type="checkbox"
              id="mic-toggle"
              checked={settings.micEnabled}
              onChange={handleMicToggle}
              className="toggle-input"
              aria-label="Enable microphone"
            />
            <label htmlFor="mic-toggle" className="toggle-label">
              🎤 Microphone
            </label>
          </div>
        </div>

        {/* Audio Device Selector - Only show when mic is enabled */}
        {settings.micEnabled && (
          <div className="setting-group">
            <label htmlFor="audio-device-select" className="setting-label">
              Audio Input
            </label>
            <select
              id="audio-device-select"
              value={settings.selectedAudioDevice || ''}
              onChange={(e) => handleAudioDeviceChange(e.target.value)}
              className="audio-device-select"
              disabled={loadingDevices}
              aria-label="Select audio input device"
            >
              {loadingDevices ? (
                <option>Loading devices...</option>
              ) : audioDevices.length === 0 ? (
                <option>No devices found</option>
              ) : (
                audioDevices.map((device) => (
                  <option key={device.id} value={device.id}>
                    {getAudioDeviceLabel(device)}
                  </option>
                ))
              )}
            </select>
          </div>
        )}
      </div>

      {/* Hotkey instruction */}
      <div className="hotkey-instruction">
        <div className="hotkey-badge">
          {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+Shift+R
        </div>
        <span className="instruction-text">Hold to record</span>
      </div>
    </div>
  );
}
