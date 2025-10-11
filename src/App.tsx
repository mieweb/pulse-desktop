import { useRecording } from './hooks/useRecording';
import { useSettings } from './hooks/useSettings';
import { StatusChip } from './components/StatusChip';
import { SettingsPanel } from './components/SettingsPanel';
import './App.css';

function App() {
  const recordingState = useRecording();
  const { settings, updateSettings } = useSettings();

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
          ✅ Saved: {recordingState.currentClipPath}
        </div>
      )}
    </main>
  );
}

export default App;
