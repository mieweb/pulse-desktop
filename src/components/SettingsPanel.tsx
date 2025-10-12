import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { AppSettings, CaptureMode, AspectRatio } from '../types';
import './SettingsPanel.css';

interface SettingsPanelProps {
  settings: AppSettings;
  onSettingsChange: (settings: Partial<AppSettings>) => void;
  onCaptureModeChange?: (mode: CaptureMode) => void;
  clipCount: number;
}

export function SettingsPanel({
  settings,
  onSettingsChange,
  onCaptureModeChange,
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

  return (
    <div className="settings-panel">
      {/* Status bar: Output location and clips counter */}
      <div className="status-bar">
        <div className="status-item">
          <div className="status-label">Output</div>
          <button
            onClick={handleSelectFolder}
            className="folder-button"
            aria-label="Select output folder"
            title={settings.outputFolder}
          >
            📁 <span className="folder-path">{settings.outputFolder.split('/').pop() || 'Movies'}</span>
          </button>
        </div>
        <div className="status-item clips-status">
          <div className="status-label">Clips</div>
          <div className="clip-counter" aria-live="polite">
            {clipCount}
          </div>
        </div>
      </div>

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
      </div>

      {/* Options and controls */}
      <div className="options-bar">
        <div className="options-group">
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
        <button
          onClick={handleAuthorizeCapture}
          className="authorize-button"
          aria-label="Authorize screen capture"
        >
          🔒 Authorize Capture
        </button>
      </div>

      {/* Hotkey instruction */}
      <div className="hotkey-instruction">
        <div className="hotkey-badge">
          {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+Shift+R
        </div>
        <span className="instruction-text">Hold to record • Release to save</span>
      </div>
    </div>
  );
}
