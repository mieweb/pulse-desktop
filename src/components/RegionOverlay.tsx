import { useState, useEffect, useRef, useCallback } from 'react';
import type { CaptureRegion, AspectRatio } from '../types';
import './RegionOverlay.css';

interface RegionOverlayProps {
  isVisible: boolean;
  aspectRatio: AspectRatio;
  scaleToPreset: boolean;
  onRegionSelected: (region: CaptureRegion) => void;
  onCancel: () => void;
}

interface DragState {
  isDragging: boolean;
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
}

export function RegionOverlay({
  isVisible,
  aspectRatio,
  scaleToPreset,
  onRegionSelected,
  onCancel,
}: RegionOverlayProps) {
  const [dragState, setDragState] = useState<DragState>({
    isDragging: false,
    startX: 0,
    startY: 0,
    currentX: 0,
    currentY: 0,
  });

  const overlayRef = useRef<HTMLDivElement>(null);

  // Focus the overlay when it becomes visible
  useEffect(() => {
    if (isVisible && overlayRef.current) {
      // Small delay to ensure the overlay is fully rendered
      setTimeout(() => {
        if (overlayRef.current) {
          overlayRef.current.focus();
          console.log('RegionOverlay focused');
        }
      }, 100);
    }
  }, [isVisible]);

  // Calculate aspect ratio constraint
  const getAspectRatio = useCallback(() => {
    switch (aspectRatio) {
      case '16:9': return 16 / 9;
      case '9:16': return 9 / 16;
      case 'none': return null;
      default: return null;
    }
  }, [aspectRatio]);

  // Constrain selection to aspect ratio
  const constrainToAspectRatio = useCallback((
    startX: number,
    startY: number,
    endX: number,
    endY: number
  ) => {
    const targetRatio = getAspectRatio();
    if (!targetRatio) return { endX, endY };

    const width = Math.abs(endX - startX);
    const height = Math.abs(endY - startY);
    const currentRatio = width / height;

    let constrainedWidth = width;
    let constrainedHeight = height;

    if (currentRatio > targetRatio) {
      // Too wide, constrain width
      constrainedWidth = height * targetRatio;
    } else {
      // Too tall, constrain height  
      constrainedHeight = width / targetRatio;
    }

    const newEndX = startX + (endX > startX ? constrainedWidth : -constrainedWidth);
    const newEndY = startY + (endY > startY ? constrainedHeight : -constrainedHeight);

    return { endX: newEndX, endY: newEndY };
  }, [getAspectRatio]);

  // Calculate selection rectangle
  const getSelectionRect = useCallback(() => {
    if (!dragState.isDragging) return null;

    let { startX, startY, currentX, currentY } = dragState;

    // Apply aspect ratio constraint
    if (aspectRatio !== 'none') {
      const constrained = constrainToAspectRatio(startX, startY, currentX, currentY);
      currentX = constrained.endX;
      currentY = constrained.endY;
    }

    const left = Math.min(startX, currentX);
    const top = Math.min(startY, currentY);
    const width = Math.abs(currentX - startX);
    const height = Math.abs(currentY - startY);

    return { left, top, width, height };
  }, [dragState, aspectRatio, constrainToAspectRatio]);

  // Calculate output resolution
  const getOutputResolution = useCallback(() => {
    const rect = getSelectionRect();
    if (!rect) return null;

    if (!scaleToPreset || aspectRatio === 'none') {
      return { width: Math.round(rect.width), height: Math.round(rect.height) };
    }

    // Scale to preset resolution
    const presets = {
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
    };

    const ratioPresets = presets[aspectRatio as keyof typeof presets];
    if (!ratioPresets) return { width: Math.round(rect.width), height: Math.round(rect.height) };

    // Find closest preset by total pixels
    const capturePixels = rect.width * rect.height;
    const closest = ratioPresets.reduce((prev, current) => {
      const prevDiff = Math.abs(prev.width * prev.height - capturePixels);
      const currentDiff = Math.abs(current.width * current.height - capturePixels);
      return currentDiff < prevDiff ? current : prev;
    });

    return closest;
  }, [getSelectionRect, scaleToPreset, aspectRatio]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!overlayRef.current) return;

    const rect = overlayRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setDragState({
      isDragging: true,
      startX: x,
      startY: y,
      currentX: x,
      currentY: y,
    });
  }, []);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragState.isDragging || !overlayRef.current) return;

    const rect = overlayRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setDragState(prev => ({
      ...prev,
      currentX: x,
      currentY: y,
    }));
  }, [dragState.isDragging]);

  const handleMouseUp = useCallback(() => {
    if (!dragState.isDragging) return;

    const rect = getSelectionRect();
    if (rect && rect.width > 20 && rect.height > 20) {
      // Convert to screen coordinates and create region
      const region: CaptureRegion = {
        x: Math.round(rect.left),
        y: Math.round(rect.top),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
      };

      onRegionSelected(region);
    }

    setDragState({
      isDragging: false,
      startX: 0,
      startY: 0,
      currentX: 0,
      currentY: 0,
    });
  }, [dragState.isDragging, getSelectionRect, onRegionSelected]);

  // Handle keyboard events - using multiple listeners for reliability
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      console.log('Key pressed in overlay:', e.key, 'target:', e.target);
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        console.log('Escape pressed, calling onCancel');
        onCancel();
      }
    };

    const handleWindowKeyDown = (e: KeyboardEvent) => {
      console.log('Window key pressed:', e.key);
      if (e.key === 'Escape' && isVisible) {
        e.preventDefault();
        e.stopPropagation();
        console.log('Window Escape pressed, calling onCancel');
        onCancel();
      }
    };

    if (isVisible) {
      // Add multiple event listeners for maximum coverage
      document.addEventListener('keydown', handleKeyDown, { capture: true });
      window.addEventListener('keydown', handleWindowKeyDown, { capture: true });
      
      return () => {
        document.removeEventListener('keydown', handleKeyDown, { capture: true });
        window.removeEventListener('keydown', handleWindowKeyDown, { capture: true });
      };
    }
  }, [isVisible, onCancel]);

  if (!isVisible) return null;

  const selectionRect = getSelectionRect();
  const outputRes = getOutputResolution();

  return (
    <div
      ref={overlayRef}
      className="region-overlay"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onKeyDown={(e) => {
        console.log('Div keydown:', e.key);
        if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          console.log('Div Escape pressed, calling onCancel');
          onCancel();
        }
      }}
      role="dialog"
      aria-label="Region selection overlay"
      tabIndex={0}
    >
      {/* Dark overlay with transparent selection area */}
      <div className="overlay-backdrop" />
      
      {/* Selection rectangle */}
      {selectionRect && (
        <div
          className="selection-rect"
          style={{
            left: selectionRect.left,
            top: selectionRect.top,
            width: selectionRect.width,
            height: selectionRect.height,
          }}
        >
          <div className="selection-border" />
          <div className="selection-handles">
            <div className="handle top-left" />
            <div className="handle top-right" />
            <div className="handle bottom-left" />
            <div className="handle bottom-right" />
          </div>
        </div>
      )}

      {/* Instructions */}
      <div className="overlay-instructions">
        <div className="instruction-panel">
          <h3>Select Capture Region</h3>
          <p>Click and drag to select the area to record</p>
          {aspectRatio !== 'none' && (
            <p className="aspect-info">Constraining to {aspectRatio} aspect ratio</p>
          )}
          {outputRes && (
            <p className="resolution-info">
              Output: {outputRes.width} × {outputRes.height}
              {scaleToPreset && aspectRatio !== 'none' && ' (scaled)'}
            </p>
          )}
        </div>
      </div>

      {/* Action buttons */}
      <div className="overlay-actions">
        <button
          onClick={onCancel}
          className="cancel-button"
          aria-label="Cancel region selection"
        >
          ✕ Cancel
        </button>
        <div className="shortcut-hint">Press Esc to cancel</div>
      </div>
    </div>
  );
}