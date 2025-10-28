import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { usePreInitStatus } from '../hooks/usePreInitStatus';
import { useRecording } from '../hooks/useRecording';
import { useActivity } from '../hooks/useActivity';
import type { PreInitStatus, RecordingStatus } from '../types';
import './StatusIndicator.css';

interface StatusIndicatorProps {
  className?: string;
}

const PRE_INIT_LABELS: Record<PreInitStatus, string> = {
  NotInitialized: 'Idle',
  Initializing: 'Waking...',
  Ready: 'Ready',
  ShuttingDown: 'Sleeping...',
};

const RECORDING_LABELS: Record<RecordingStatus, string> = {
  idle: 'Idle',
  recording: 'Recording',
  saving: 'Saving...',
  error: 'Error',
};

const PRE_INIT_DESCRIPTIONS: Record<PreInitStatus, string> = {
  NotInitialized: 'Click to pre-initialize capturer for fast recording startup (~3s)',
  Initializing: 'Pre-initializing capturer for fast recording startup...',
  Ready: 'Click to shut down pre-initialized capturer and save resources',
  ShuttingDown: 'Capturer shutting down...',
};

export function StatusIndicator({ className = '' }: StatusIndicatorProps) {
  const { updateActivity } = useActivity();
  const { status: preInitStatus, loading: preInitLoading } = usePreInitStatus();
  const recordingState = useRecording();
  const [isToggling, setIsToggling] = useState(false);

  const handleClick = async () => {
    updateActivity();
    
    // Don't allow clicking during recording or transitions
    if (isToggling || recordingState.status !== 'idle' || 
        preInitStatus === 'Initializing' || preInitStatus === 'ShuttingDown') {
      return;
    }
    
    try {
      setIsToggling(true);
      const newStatus = await invoke<string>('toggle_pre_init');
      console.log('🔄 Pre-init toggled to:', newStatus);
    } catch (err) {
      console.error('❌ Failed to toggle pre-init:', err);
    } finally {
      setIsToggling(false);
    }
  };

  // Determine the current display state
  const getDisplayState = () => {
    if (preInitLoading) {
      return {
        label: 'Loading...',
        description: 'Loading capturer status...',
        cssClass: 'loading',
        isClickable: false
      };
    }

    // Recording status takes priority over pre-init status
    if (recordingState.status === 'recording') {
      return {
        label: RECORDING_LABELS.recording,
        description: 'Currently recording. Release hotkey to stop.',
        cssClass: 'recording-active',
        isClickable: false
      };
    }

    if (recordingState.status === 'saving') {
      return {
        label: RECORDING_LABELS.saving,
        description: 'Saving recording to disk...',
        cssClass: 'recording-saving',
        isClickable: false
      };
    }

    if (recordingState.status === 'error') {
      return {
        label: RECORDING_LABELS.error,
        description: recordingState.error || 'Recording error occurred',
        cssClass: 'recording-error',
        isClickable: false
      };
    }

    // Show pre-init status when idle
    const preInitClass = `pre-init-${preInitStatus.toLowerCase().replace(/([A-Z])/g, '-$1').toLowerCase()}`;
    const isClickable = !isToggling && (preInitStatus === 'NotInitialized' || preInitStatus === 'Ready');
    
    return {
      label: PRE_INIT_LABELS[preInitStatus],
      description: PRE_INIT_DESCRIPTIONS[preInitStatus],
      cssClass: preInitClass,
      isClickable
    };
  };

  const displayState = getDisplayState();

  return (
    <div 
      className={`status-indicator ${displayState.cssClass} ${displayState.isClickable ? 'clickable' : 'disabled'} ${className}`}
      onClick={handleClick}
      title={`${displayState.description} ${displayState.isClickable ? '(Click to toggle)' : ''}`}
      role="button"
      tabIndex={displayState.isClickable ? 0 : -1}
      aria-label={`${displayState.label}. ${displayState.description} ${displayState.isClickable ? 'Click to toggle state.' : ''}`}
    >
      <div className="status-icon" />
      <span className="status-text">{displayState.label}</span>
    </div>
  );
}