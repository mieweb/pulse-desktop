import { useState } from 'react';
import './DebugControls.css';

interface DebugControlsProps {
  onDragDropChange: (enabled: boolean) => void;
  onAriaFocusChange: (enabled: boolean) => void;
  dragDropEnabled: boolean;
  ariaFocusEnabled: boolean;
}

export default function DebugControls({ 
  onDragDropChange, 
  onAriaFocusChange, 
  dragDropEnabled, 
  ariaFocusEnabled 
}: DebugControlsProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div className={`debug-controls ${isExpanded ? 'expanded' : ''}`}>
      <button
        className="debug-toggle"
        onClick={() => setIsExpanded(!isExpanded)}
        title="Toggle debug controls"
        aria-label="Toggle debug controls"
        aria-expanded={isExpanded}
      >
        🐛 Debug
      </button>

      {isExpanded && (
        <div className="debug-panel">
          <div className="debug-section">
            <h4>Debug Logging</h4>
            
            <label className="debug-option">
              <input
                type="checkbox"
                checked={dragDropEnabled}
                onChange={(e) => onDragDropChange(e.target.checked)}
              />
              <span>
                🎯 Drag & Drop
                <small>Log drag start, drag over, and drop events</small>
              </span>
            </label>

            <label className="debug-option">
              <input
                type="checkbox"
                checked={ariaFocusEnabled}
                onChange={(e) => onAriaFocusChange(e.target.checked)}
              />
              <span>
                ⌨️ ARIA & Focus
                <small>Log keyboard navigation and focus changes</small>
              </span>
            </label>
          </div>

          <div className="debug-info">
            <p>
              <strong>Console Prefixes:</strong>
            </p>
            <ul>
              <li><code>🎯 [DRAG]</code> - Drag & drop events</li>
              <li><code>⌨️ [KEYBOARD]</code> - Keyboard shortcuts</li>
              <li><code>👁️ [FOCUS]</code> - Focus changes</li>
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}
