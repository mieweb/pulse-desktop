import { useState, useEffect, useMemo } from 'react';
import { useRecording } from './hooks/useRecording';
import { useSettings } from './hooks/useSettings';

import { SettingsPanel } from './components/SettingsPanel';
import { RegionOverlay } from './components/RegionOverlay';
import { ProjectPanel } from './components/ProjectPanel';
import { ProjectNameModal } from './components/ProjectNameModal';
import DebugControls from './components/DebugControls';
import { useProjects } from './hooks/useProjects';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { CaptureRegion, ClipSavedEvent } from './types';
import './App.css';

function App() {
  const recordingState = useRecording();
  const { settings, updateSettings } = useSettings();
  const { projects, currentProject, refreshProjects, createProject, setCurrentProject } = useProjects();

  // Debug flags
  const [debugDragDrop, setDebugDragDrop] = useState(false);
  const [debugAriaFocus, setDebugAriaFocus] = useState(false);

  // Project name modal state
  const [showProjectModal, setShowProjectModal] = useState(false);
  const [projectModalError, setProjectModalError] = useState<string | undefined>();

  // Calculate current project's clip count
  const currentClipCount = useMemo(() => {
    if (!currentProject) return 0;
    const project = projects.find(p => p.name === currentProject);
    return project?.videoCount || 0;
  }, [currentProject, projects]);

  const [isRegionSelectorMode, setIsRegionSelectorMode] = useState(false);
  const [regionSelectorConfig, setRegionSelectorConfig] = useState<{
    aspectRatio: string;
    scaleToPreset: boolean;
  }>({ aspectRatio: 'none', scaleToPreset: false });

  // Listen for clip saved events to refresh project data
  useEffect(() => {
    const unsubscribe = listen<ClipSavedEvent>('clip-saved', async () => {
      // Refresh projects when a clip is saved to update video counts
      await refreshProjects();
    });

    return () => {
      unsubscribe.then((fn: () => void) => fn());
    };
  }, [refreshProjects]);

  // Listen for project-required event (when recording starts without a project)
  useEffect(() => {
    const unsubscribe = listen('project-required', () => {
      console.log('Project required event received - showing modal');
      setShowProjectModal(true);
      setProjectModalError(undefined);
    });

    return () => {
      unsubscribe.then((fn: () => void) => fn());
    };
  }, []);

  // Handle project creation from modal
  const handleProjectSubmit = async (projectName: string) => {
    try {
      setProjectModalError(undefined);
      await createProject(projectName);
      await setCurrentProject(projectName);
      setShowProjectModal(false);
      console.log('Project created and set:', projectName);
    } catch (err) {
      console.error('Failed to create project:', err);
      setProjectModalError(err as string);
    }
  };

  const handleProjectCancel = () => {
    setShowProjectModal(false);
    setProjectModalError(undefined);
  };

  // Check if this is the region selector window
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const mode = params.get('mode');
    
    if (mode === 'region-selector') {
      setIsRegionSelectorMode(true);
      setRegionSelectorConfig({
        aspectRatio: params.get('aspectRatio') || 'none',
        scaleToPreset: params.get('scaleToPreset') === 'true',
      });
      
      // Set body class for region selector styling
      document.body.classList.add('region-selector-mode');
      document.body.style.background = 'transparent';
      document.body.style.margin = '0';
      document.body.style.padding = '0';
      document.body.style.overflow = 'hidden';
    }
  }, []);

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

  const handleRegionSelected = async (region: CaptureRegion) => {
    console.log('handleRegionSelected called with:', region);
    console.log('isRegionSelectorMode:', isRegionSelectorMode);
    
    try {
      await invoke('set_capture_region', {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
      });
      
      if (isRegionSelectorMode) {
        console.log('Closing region selector window...');
        await invoke('close_region_selector');
        console.log('Window close command sent');
      }
      
      console.log('Region set:', region);
    } catch (error) {
      console.error('Failed to set capture region:', error);
    }
  };

  const handleRegionCancel = async () => {
    console.log('handleRegionCancel called');
    console.log('isRegionSelectorMode:', isRegionSelectorMode);
    
    if (isRegionSelectorMode) {
      // Close the region selector window and clear region
      try {
        console.log('Clearing region and closing window...');
        await invoke('clear_capture_region');
        await invoke('close_region_selector');
        console.log('Cancel window close command sent');
      } catch (error) {
        console.error('Failed to cancel region selection:', error);
      }
    } else {
      // If user cancels, clear any existing region and return to full screen
      if (settings.captureMode === 'region') {
        updateSettings({ captureMode: 'full' });
        try {
          await invoke('clear_capture_region');
        } catch (error) {
          console.error('Failed to clear capture region:', error);
        }
      }
    }
  };

  const handleCaptureModeChange = async (mode: 'full' | 'region') => {
    updateSettings({ captureMode: mode });
    
    if (mode === 'region') {
      // Open region selection as a new window covering the entire screen
      try {
        await invoke('open_region_selector', {
          aspectRatio: settings.aspectRatio,
          scaleToPreset: settings.scaleToPreset,
        });
      } catch (error) {
        console.error('Failed to open region selector:', error);
      }
    } else {
      // Clear region when switching to full screen
      try {
        await invoke('clear_capture_region');
      } catch (error) {
        console.error('Failed to clear capture region:', error);
      }
    }
  };

  // If this is the region selector window, only show the overlay
  if (isRegionSelectorMode) {
    return (
      <RegionOverlay
        isVisible={true}
        aspectRatio={regionSelectorConfig.aspectRatio as any}
        scaleToPreset={regionSelectorConfig.scaleToPreset}
        onRegionSelected={handleRegionSelected}
        onCancel={handleRegionCancel}
      />
    );
  }

  return (
    <main className="container">
      {/* Project Name Modal */}
      <ProjectNameModal
        isVisible={showProjectModal}
        onSubmit={handleProjectSubmit}
        onCancel={handleProjectCancel}
        error={projectModalError}
      />

      <header className="app-header">
        <h1>🎬 Pulse Desktop</h1>
        <div 
          className={`recording-status status-${recordingState.status}`}
          role="status"
          aria-live="polite"
          aria-label={`Recording status: ${recordingState.status}`}
        >
          <span className="status-indicator" />
          <span className="status-text">
            {recordingState.status === 'idle' && 'Idle'}
            {recordingState.status === 'recording' && 'Recording'}
            {recordingState.status === 'saving' && 'Saving...'}
            {recordingState.status === 'error' && 'Error'}
          </span>
        </div>
      </header>

      <div className="main-controls">
        <ProjectPanel 
          clipCount={currentClipCount}
          outputFolder={settings.outputFolder}
          debugDragDrop={debugDragDrop}
          debugAriaFocus={debugAriaFocus}
        />

        <SettingsPanel
          settings={settings}
          onSettingsChange={updateSettings}
          onCaptureModeChange={handleCaptureModeChange}
        />
      </div>

      {/* Debug Controls */}
      <DebugControls
        dragDropEnabled={debugDragDrop}
        ariaFocusEnabled={debugAriaFocus}
        onDragDropChange={setDebugDragDrop}
        onAriaFocusChange={setDebugAriaFocus}
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
