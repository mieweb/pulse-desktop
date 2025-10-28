import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TimelineEntry } from '../types';
import { useUndoRedo } from '../hooks/useUndoRedo';
import './ClipsList.css';

interface ClipsListProps {
  projectName: string | null;
  outputFolder: string;
  onClipsUpdate?: (count: number) => void;
  debugDragDrop?: boolean;
  debugAriaFocus?: boolean;
}

// Drag-drop reordering with visual drop indicator
export default function ClipsList({ 
  projectName, 
  outputFolder, 
  onClipsUpdate,
  debugAriaFocus = false
}: ClipsListProps) {
  const { state: clips, setState: setClips, canUndo, canRedo, undo, redo } = useUndoRedo<TimelineEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [draggedClipId, setDraggedClipId] = useState<string | null>(null);
  const [dropIndicatorPosition, setDropIndicatorPosition] = useState<number | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editLabel, setEditLabel] = useState('');
  const [focusedClipId, setFocusedClipId] = useState<string | null>(null);

  // Debounced save to backend
  const saveTimeline = async (updatedClips: TimelineEntry[]) => {
    if (!projectName) return;

    try {
      // Get the full timeline structure
      const timeline = await invoke<{ entries: TimelineEntry[]; projectName: string; createdAt: string; lastModified: string; metadata: any }>('get_project_timeline', { projectName });
      
      // Update with new clips
      timeline.entries = updatedClips;
      
      await invoke('save_project_timeline', { 
        projectName, 
        timeline 
      });
    } catch (err) {
      console.error('Failed to save timeline:', err);
    }
  };

  // Load clips when project changes
  useEffect(() => {
    if (!projectName) {
      setClips([]);
      return;
    }

    const loadClips = async () => {
      setLoading(true);
      try {
        const timeline = await invoke<{ entries: TimelineEntry[] }>('get_project_timeline', { projectName });
        // Filter out soft-deleted clips and sort by recordedAt (newest first)
        const activeClips = timeline.entries.filter(clip => !clip.deleted);
        const sorted = [...activeClips].sort((a, b) => 
          new Date(b.recordedAt).getTime() - new Date(a.recordedAt).getTime()
        );
        setClips(sorted, false); // false = don't create undo checkpoint for initial load
        onClipsUpdate?.(sorted.length);
      } catch (err) {
        console.error('Failed to load clips:', err);
        setClips([], false);
      } finally {
        setLoading(false);
      }
    };

    loadClips();
  }, [projectName, onClipsUpdate]);

  // Mouse-based drag and drop handlers (fallback for WebKit drag-drop issues)
  const handleMouseDown = (e: React.MouseEvent, clipId: string) => {
    // Only left mouse button
    if (e.button !== 0) return;
    
    // Prevent text selection during drag
    e.preventDefault();
    
    setDraggedClipId(clipId);
    setDropIndicatorPosition(null);
  };

  const handleMouseMove = (e: React.MouseEvent, targetClipId: string, targetIndex: number) => {
    // Prevent default to avoid text selection
    e.preventDefault();
    
    if (draggedClipId === null || draggedClipId === targetClipId) {
      return;
    }

    // Work with the visible clips array for drag feedback
    const visibleClips = clips.filter(c => !c.deleted);
    const draggedVisibleIdx = visibleClips.findIndex(c => c.id === draggedClipId);
    
    if (draggedVisibleIdx === -1) {
      return;
    }

    // Calculate where to show the drop indicator
    // If dragging down (draggedIdx < targetIdx), show indicator AFTER target
    // If dragging up (draggedIdx > targetIdx), show indicator BEFORE target
    const dropPosition = draggedVisibleIdx < targetIndex ? targetIndex : targetIndex;

    // Update visual feedback
    setDropIndicatorPosition(dropPosition);
  };

  const handleMouseUp = async () => {
    if (draggedClipId === null) {
      return;
    }

    // Store draggedClipId before clearing it
    const draggedId = draggedClipId;
    const dropPos = dropIndicatorPosition;
    
    // Clear drag state immediately to prevent duplicate handling
    setDraggedClipId(null);
    setDropIndicatorPosition(null);

    // If we have a drop position, reorder to that position now
    if (draggedId !== null && dropPos !== null && projectName) {
      const draggedClip = clips.find(c => c.id === draggedId);
      
      if (draggedClip) {
        // Remove dragged clip from array
        const withoutDragged = clips.filter(c => c.id !== draggedId);
        const visibleWithoutDragged = withoutDragged.filter(c => !c.deleted);
        
        // Insert at drop position in visible array
        const targetVisibleIndex = Math.min(dropPos, visibleWithoutDragged.length);
        
        // Find the actual index in full array
        let insertIndex = withoutDragged.length; // Default to end
        let visibleCount = 0;
        for (let i = 0; i < withoutDragged.length; i++) {
          if (!withoutDragged[i].deleted) {
            if (visibleCount === targetVisibleIndex) {
              insertIndex = i;
              break;
            }
            visibleCount++;
          }
        }
        
        // Insert at calculated position
        const newClips = [...withoutDragged];
        newClips.splice(insertIndex, 0, draggedClip);
        
        // Create undo checkpoint
        setClips(newClips, true);
        
        await saveTimeline(newClips);
      }
    }
  };

  // Label editing handlers
  const handleLabelEdit = (clip: TimelineEntry) => {
    setEditingId(clip.id);
    setEditLabel(clip.label || clip.filename);
  };

  const handleLabelSave = async (clip: TimelineEntry) => {
    if (!projectName) return;

    const updatedClips = clips.map(c =>
      c.id === clip.id ? { ...c, label: editLabel } : c
    );

    setClips(updatedClips); // Creates undo checkpoint
    setEditingId(null);

    // Save to backend
    await saveTimeline(updatedClips);
  };

  const handleLabelCancel = () => {
    setEditingId(null);
    setEditLabel('');
  };

  // Keyboard navigation handlers
  const handleKeyDown = (e: React.KeyboardEvent, clip: TimelineEntry, visibleClips: TimelineEntry[]) => {
    const currentIndex = visibleClips.findIndex(c => c.id === clip.id);
    
    if (debugAriaFocus) {
      console.log('⌨️ [KEYBOARD]:', {
        key: e.key,
        metaKey: e.metaKey,
        ctrlKey: e.ctrlKey,
        currentClip: clip.label || clip.filename,
        currentIndex,
        totalVisible: visibleClips.length
      });
    }
    
    // Arrow key navigation
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      const nextIndex = Math.min(currentIndex + 1, visibleClips.length - 1);
      const nextClip = visibleClips[nextIndex];
      
      if (debugAriaFocus) {
        console.log('⌨️ [KEYBOARD] Navigate Down:', {
          from: currentIndex,
          to: nextIndex,
          toClip: nextClip.label || nextClip.filename
        });
      }
      
      setFocusedClipId(nextClip.id);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      const prevIndex = Math.max(currentIndex - 1, 0);
      const prevClip = visibleClips[prevIndex];
      
      if (debugAriaFocus) {
        console.log('⌨️ [KEYBOARD] Navigate Up:', {
          from: currentIndex,
          to: prevIndex,
          toClip: prevClip.label || prevClip.filename
        });
      }
      
      setFocusedClipId(prevClip.id);
    }
    // Ctrl+Arrow for reordering
    else if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowDown') {
      e.preventDefault();
      if (currentIndex < visibleClips.length - 1) {
        if (debugAriaFocus) {
          console.log('⌨️ [KEYBOARD] Move clip down:', clip.label || clip.filename);
        }
        moveClip(clip.id, 'down');
      }
    } else if ((e.metaKey || e.ctrlKey) && e.key === 'ArrowUp') {
      e.preventDefault();
      if (currentIndex > 0) {
        if (debugAriaFocus) {
          console.log('⌨️ [KEYBOARD] Move clip up:', clip.label || clip.filename);
        }
        moveClip(clip.id, 'up');
      }
    }
    // Enter to play clip
    else if (e.key === 'Enter') {
      e.preventDefault();
      if (debugAriaFocus) {
        console.log('⌨️ [KEYBOARD] Play clip:', clip.label || clip.filename);
      }
      handleOpen(clip);
    }
    // Space to select/focus
    else if (e.key === ' ') {
      e.preventDefault();
      if (debugAriaFocus) {
        console.log('⌨️ [KEYBOARD] Focus clip:', clip.label || clip.filename);
      }
      setFocusedClipId(clip.id);
    }
    // Delete key (only if not editing)
    else if ((e.key === 'Delete' || e.key === 'Backspace') && editingId !== clip.id) {
      e.preventDefault();
      if (debugAriaFocus) {
        console.log('⌨️ [KEYBOARD] Delete clip:', clip.label || clip.filename);
      }
      handleDelete(clip);
    }
  };

  // Move clip up or down (keyboard reordering)
  const moveClip = async (clipId: string, direction: 'up' | 'down') => {
    const clipIndex = clips.findIndex(c => c.id === clipId);
    if (clipIndex === -1) return;

    const newIndex = direction === 'up' ? clipIndex - 1 : clipIndex + 1;
    if (newIndex < 0 || newIndex >= clips.length) return;

    const newClips = [...clips];
    const [movedClip] = newClips.splice(clipIndex, 1);
    newClips.splice(newIndex, 0, movedClip);

    setClips(newClips, true); // Create undo checkpoint
    await saveTimeline(newClips);
  };

  // Delete clip handler (soft delete)
  const handleDelete = async (clip: TimelineEntry) => {
    if (!projectName || !confirm(`Delete ${clip.label || clip.filename}?`)) return;

    // Mark as deleted instead of removing from array
    const updatedClips = clips.map(c =>
      c.id === clip.id 
        ? { ...c, deleted: true, deletedAt: new Date().toISOString() }
        : c
    );

    // Filter out deleted clips for display, but keep in state for undo
    const visibleClips = updatedClips.filter(c => !c.deleted);
    
    setClips(updatedClips); // Creates undo checkpoint with full data
    onClipsUpdate?.(visibleClips.length);

    // Save to backend (including soft-deleted clips)
    await saveTimeline(updatedClips);
  };

  // Open clip in system player
  const handleOpen = async (clip: TimelineEntry) => {
    if (!projectName) return;

    try {
      // Construct full path to the clip file
      const clipPath = `${outputFolder}/${projectName}/${clip.filename}`;
      await invoke('open_file', { path: clipPath });
    } catch (err) {
      console.error('Failed to open clip:', err);
    }
  };

  // Format duration (milliseconds to MM:SS)
  const formatDuration = (durationMs: number): string => {
    if (!durationMs || isNaN(durationMs) || durationMs < 0) {
      return "0:00";
    }
    const totalSeconds = Math.round(durationMs / 1000);
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  // Format date
  const formatDate = (recordedAt: string): string => {
    const date = new Date(recordedAt);
    return date.toLocaleString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
      hour12: true
    });
  };

  if (!projectName) {
    return (
      <div className="clips-list empty">
        <p className="empty-state">Select a project to view clips</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="clips-list loading">
        <p>Loading clips...</p>
      </div>
    );
  }

  // Filter visible clips (not deleted)
  const visibleClips = clips.filter(c => !c.deleted);

  if (visibleClips.length === 0) {
    return (
      <div className="clips-list empty">
        <p className="empty-state">No clips yet. Start recording!</p>
      </div>
    );
  }

  return (
    <div className="clips-list" role="region" aria-label="Video clips timeline">
      <div className="clips-header">
        <span className="clips-count" aria-live="polite">
          {visibleClips.length} clip{visibleClips.length !== 1 ? 's' : ''}
        </span>
        <div className="undo-redo-controls" role="group" aria-label="Undo and redo controls">
          <button
            className="undo-redo-btn"
            onClick={undo}
            disabled={!canUndo}
            title="Undo (⌘Z)"
            aria-label="Undo last action"
            aria-keyshortcuts="Meta+Z"
          >
            ↶
          </button>
          <button
            className="undo-redo-btn"
            onClick={redo}
            disabled={!canRedo}
            title="Redo (⌘Y)"
            aria-label="Redo last action"
            aria-keyshortcuts="Meta+Y"
          >
            ↷
          </button>
        </div>
      </div>
      <div 
        className="clips-container" 
        role="list" 
        aria-label="Timeline clips"
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        <div id="clips-instructions" className="sr-only">
          Use arrow keys to navigate, Command+Arrow to reorder, Enter to play, Delete to remove
        </div>
        {visibleClips.map((clip, index) => (
          <div 
            key={clip.id} 
            className="clip-wrapper"
            onMouseMove={(e) => handleMouseMove(e, clip.id, index)}
            onMouseUp={handleMouseUp}
          >
            {/* Drop indicator - show BEFORE this clip if drop position matches */}
            {dropIndicatorPosition === index && (
              <div className="drop-indicator" aria-hidden="true">
                <div className="drop-indicator-line"></div>
              </div>
            )}
            
            <div
              role="listitem"
              tabIndex={0}
              aria-label={`Clip ${index + 1}: ${clip.label || clip.filename}, duration ${formatDuration(clip.durationMs)}`}
              aria-describedby="clips-instructions"
              className={`clip-row ${draggedClipId === clip.id ? 'dragging' : ''} ${focusedClipId === clip.id ? 'focused' : ''}`}
              onMouseDown={(e) => handleMouseDown(e, clip.id)}
              onKeyDown={(e) => handleKeyDown(e, clip, visibleClips)}
              onFocus={() => {
                if (debugAriaFocus) {
                  console.log('👁️ [FOCUS] Clip focused:', {
                    clipId: clip.id,
                    clipLabel: clip.label || clip.filename,
                    index,
                    totalVisible: visibleClips.length
                  });
                }
                setFocusedClipId(clip.id);
              }}
            >
            {/* Drag handle */}
            <div className="clip-drag-handle" title="Drag to reorder">
              <span>⋮⋮</span>
            </div>

            {/* Thumbnail */}
            <div className="clip-thumbnail">
              {clip.thumbnail ? (
                <img
                  src={`${outputFolder}/${projectName}/${clip.thumbnail}`}
                  alt={clip.label || clip.filename}
                />
              ) : (
                <div className="clip-thumbnail-placeholder">
                  <span>🎬</span>
                </div>
              )}
            </div>

            {/* Details */}
            <div className="clip-details">
              {editingId === clip.id ? (
                <input
                  type="text"
                  className="clip-label-input"
                  value={editLabel}
                  onChange={(e) => setEditLabel(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleLabelSave(clip);
                    if (e.key === 'Escape') handleLabelCancel();
                  }}
                  onBlur={() => handleLabelSave(clip)}
                  autoFocus
                />
              ) : (
                <div className="clip-label" onClick={() => handleLabelEdit(clip)}>
                  {clip.label || clip.filename}
                </div>
              )}
              <div className="clip-metadata">
                <span className="clip-duration">{formatDuration(clip.durationMs)}</span>
                <span className="clip-separator">•</span>
                <span className="clip-resolution">{clip.resolution.width}×{clip.resolution.height}</span>
                <span className="clip-separator">•</span>
                <span className="clip-aspect">{clip.aspectRatio}</span>
                <span className="clip-separator">•</span>
                <span className="clip-time">{formatDate(clip.recordedAt)}</span>
                {clip.micEnabled && (
                  <>
                    <span className="clip-separator">•</span>
                    <span className="clip-audio" title="Has microphone audio">🎤</span>
                  </>
                )}
              </div>
            </div>

            {/* Actions */}
            <div className="clip-actions">
              <button
                className="clip-action-btn"
                onClick={() => handleOpen(clip)}
                title="Open in system player"
              >
                ▶️
              </button>
              <button
                className="clip-action-btn delete"
                onClick={() => handleDelete(clip)}
                title="Delete clip"
              >
                🗑️
              </button>
            </div>
          </div>
          
          {/* Drop indicator - show AFTER this clip if it's the last position */}
          {dropIndicatorPosition === index + 1 && (
            <div className="drop-indicator" aria-hidden="true">
              <div className="drop-indicator-line"></div>
            </div>
          )}
        </div>
        ))}
      </div>
    </div>
  );
}
