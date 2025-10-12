import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { AppSettings, CaptureMode, AspectRatio } from '../types';
import './SettingsPanel.css';

interface SettingsPanelProps {
  settings: AppSettings;
  onSettingsChange: (settings: Partial<AppSettings>) => void;
  clipCount: number;
}

export function SettingsPanel({
  settings,
  onSettingsChange,
  clipCount,
}: SettingsPanelProps) {
  const handleSelectFolder = async () => {
    try {
      // Get the current output folder from backend (will be absolute path)
      const currentFolder = await invoke<string>('get_output_folder');
      
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: currentFolder,
        title: 'Select Output Folder',
      });
      
      if (selected && typeof selected === 'string') {
        onSettingsChange({ outputFolder: selected });
        await invoke('set_output_folder', { path: selected });
      }
    } catch (error) {
      console.error('Failed to select folder:', error);
    }
  };

  const handleAuthorizeCapture = async () => {
    try {
      await invoke('authorize_capture');
    } catch (error) {
      console.error('Failed to authorize capture:', error);
    }
  };

  const handleCaptureModeChange = (mode: CaptureMode) => {
    onSettingsChange({ captureMode: mode });
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

  return (
    <div className="settings-panel">
      {/* Top row: Output and clips counter combined */}
      <div className="settings-section compact">
        <div className="section-row">
          <div className="section-col">
            <h2>Output Folder</h2>
            <button
              onClick={handleSelectFolder}
              className="button-secondary compact"
              aria-label="Select output folder"
              title={settings.outputFolder}
            >
              📁 Change
            </button>
          </div>
          <div className="section-col text-right">
            <h2>Saved Clips</h2>
            <div className="clip-counter-large" aria-live="polite">
              {clipCount}
            </div>
          </div>
        </div>
      </div>

      {/* Capture mode and Aspect ratio in one row */}
      <div className="settings-section compact">
        <div className="section-row">
          <div className="section-col">
            <h2>Capture</h2>
            <div className="button-group compact" role="group" aria-label="Capture mode">
              <button
                onClick={() => handleCaptureModeChange('full')}
                className={settings.captureMode === 'full' ? 'active' : ''}
                aria-pressed={settings.captureMode === 'full'}
                title="Full Screen"
              >
                🖥️ Full
              </button>
              <button
                onClick={() => handleCaptureModeChange('region')}
                className={settings.captureMode === 'region' ? 'active' : ''}
                aria-pressed={settings.captureMode === 'region'}
                title="Region"
              >
                ✂️ Region
              </button>
            </div>
          </div>
          <div className="section-col">
            <h2>Aspect Ratio</h2>
            <div className="button-group compact" role="group" aria-label="Aspect ratio">
              <button
                onClick={() => handleAspectChange('16:9')}
                className={settings.aspectRatio === '16:9' ? 'active' : ''}
                aria-pressed={settings.aspectRatio === '16:9'}
              >
                16:9
              </button>
              <button
                onClick={() => handleAspectChange('9:16')}
                className={settings.aspectRatio === '9:16' ? 'active' : ''}
                aria-pressed={settings.aspectRatio === '9:16'}
              >
                9:16
              </button>
              <button
                onClick={() => handleAspectChange('none')}
                className={settings.aspectRatio === 'none' ? 'active' : ''}
                aria-pressed={settings.aspectRatio === 'none'}
              >
                Free
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Options row: checkboxes and authorize */}
      <div className="settings-section compact">
        <div className="section-row">
          <div className="section-col">
            <label className="checkbox-label compact">
              <input
                type="checkbox"
                checked={settings.scaleToPreset}
                onChange={handleScaleToggle}
                aria-label="Scale to preset resolution"
              />
              <span>Scale to preset</span>
            </label>
            <label className="checkbox-label compact">
              <input
                type="checkbox"
                checked={settings.micEnabled}
                onChange={handleMicToggle}
                aria-label="Enable microphone"
              />
              <span>🎤 Microphone</span>
            </label>
          </div>
          <div className="section-col text-right">
            <button
              onClick={handleAuthorizeCapture}
              className="button-secondary compact"
              aria-label="Authorize screen capture"
            >
              🔒 Authorize
            </button>
          </div>
        </div>
      </div>

      {/* Hotkey info - compact footer */}
      <div className="settings-footer">
        <span className="hotkey-display">
          {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+Shift+R
        </span>
        <span className="help-text">Hold to record, release to save</span>
      </div>
    </div>
  );
}
