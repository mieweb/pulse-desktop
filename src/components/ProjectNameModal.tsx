import { useState, useEffect, useRef } from 'react';
import './ProjectNameModal.css';

interface ProjectNameModalProps {
  isVisible: boolean;
  onSubmit: (projectName: string) => void;
  onCancel: () => void;
  error?: string;
}

export function ProjectNameModal({ 
  isVisible, 
  onSubmit, 
  onCancel,
  error 
}: ProjectNameModalProps) {
  const [projectName, setProjectName] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input when modal becomes visible
  useEffect(() => {
    if (isVisible && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isVisible]);

  // Reset project name when modal opens
  useEffect(() => {
    if (isVisible) {
      setProjectName('');
    }
  }, [isVisible]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = projectName.trim();
    if (trimmed) {
      onSubmit(trimmed);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onCancel();
    }
  };

  if (!isVisible) return null;

  return (
    <div 
      className="modal-overlay" 
      onClick={onCancel}
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
    >
      <div 
        className="modal-content" 
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header">
          <h2 id="modal-title">Create Project</h2>
          <button
            className="modal-close"
            onClick={onCancel}
            aria-label="Close dialog"
            type="button"
          >
            ✕
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            <p className="modal-message">
              Please name your project to start recording:
            </p>
            
            <input
              ref={inputRef}
              type="text"
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="Enter project name..."
              className="modal-input"
              maxLength={50}
              aria-label="Project name"
              aria-required="true"
            />

            {error && (
              <div className="modal-error" role="alert">
                {error}
              </div>
            )}
          </div>

          <div className="modal-footer">
            <button
              type="button"
              onClick={onCancel}
              className="modal-btn modal-btn-cancel"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!projectName.trim()}
              className="modal-btn modal-btn-submit"
            >
              Create & Record
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
