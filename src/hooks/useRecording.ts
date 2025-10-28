import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { RecordingState, ClipSavedEvent, ErrorEvent } from '../types';

/**
 * Hook to manage recording state and listen to backend events
 */
export function useRecording() {
  const [recordingState, setRecordingState] = useState<RecordingState>({
    status: 'idle',
    clipCount: 0,
  });

  useEffect(() => {
    // Listen for status updates from Rust backend
    const unlistenStatus = listen<string>('recording-status', (event) => {
      setRecordingState((prev) => ({
        ...prev,
        status: event.payload as RecordingState['status'],
      }));
    });

    // Listen for clip saved events
    const unlistenClipSaved = listen<ClipSavedEvent>('clip-saved', (event) => {
      console.log('💾 Frontend received clip-saved:', event.payload);
      setRecordingState((prev) => ({
        ...prev,
        // Don't change status - it's already set to idle by the recording-status event
        // clipCount is now managed by the project system, not here
        currentClipPath: event.payload.path,
      }));
    });

    // Listen for error events
    const unlistenError = listen<ErrorEvent>('recording-error', (event) => {
      setRecordingState((prev) => ({
        ...prev,
        status: 'error',
        error: event.payload.message,
      }));
    });

    // Cleanup listeners
    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenClipSaved.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  return recordingState;
}
