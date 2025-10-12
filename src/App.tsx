import { useRecording } from './hooks/useRecording';
import { useSettings } from './hooks/useSettings';
import { StatusChip } from './components/StatusChip';
import { SettingsPanel } from './components/SettingsPanel';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

function App() {
  const recordingState = useRecording();
  const { settings, updateSettings } = useSettings();

  const handleOpenFolder = async () => {
    if (recordingState.currentClipPath) {
      try {
        // Extract folder path from file path
        const folderPath = recordingState.currentClipPath.substring(
          0,
          recordingState.currentClipPath.lastIndexOf('/')
        );
        await invoke('open_folder', { path: folderPath });
      } catch (error) {
        console.error('Failed to open folder:', error);
      }
    }
  };

  const handleOpenVideo = async () => {
    if (recordingState.currentClipPath) {
      try {
        await invoke('open_file', { path: recordingState.currentClipPath });
      } catch (error) {
        console.error('Failed to open video:', error);
      }
    }
  };

  return (
    <main className="container">
      <header className="app-header">
        <h1>🎬 Pulse Desktop</h1>
        <StatusChip status={recordingState.status} />
      </header>

      <SettingsPanel
        settings={settings}
        onSettingsChange={updateSettings}
        clipCount={recordingState.clipCount}
      />

      {recordingState.error && (
        <div className="error-message" role="alert" aria-live="assertive">
          <strong>Error:</strong> {recordingState.error}
        </div>
      )}

      {recordingState.currentClipPath && (
        <div className="success-message" role="status" aria-live="polite">
          <div className="success-content">
            <span>✅ Saved: {recordingState.currentClipPath.split('/').pop()}</span>
            <div className="success-actions">
              <button
                onClick={handleOpenVideo}
                className="action-button"
                aria-label="Open video file"
              >
                ▶️ Play Video
              </button>
              <button
                onClick={handleOpenFolder}
                className="action-button"
                aria-label="Open containing folder"
              >
                📁 Open Folder
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
