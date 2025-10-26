import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useProjects } from '../hooks/useProjects';
import ClipsList from './ClipsList';
import './ProjectPanel.css';

interface ProjectPanelProps {
  onProjectChange?: (projectName: string) => void;
  clipCount?: number;
  outputFolder?: string;
  debugDragDrop?: boolean;
  debugAriaFocus?: boolean;
}

export function ProjectPanel({ 
  onProjectChange, 
  clipCount = 0, 
  outputFolder = '',
  debugDragDrop = false,
  debugAriaFocus = false
}: ProjectPanelProps) {
  const { 
    projects, 
    currentProject, 
    loading, 
    error, 
    createProject, 
    setCurrentProject,
    reconcileProjectTimeline,
    refreshProjects
  } = useProjects();
  
  const [isCreatingProject, setIsCreatingProject] = useState(false);
  const [newProjectName, setNewProjectName] = useState('');
  const [createError, setCreateError] = useState<string | null>(null);
  const [reconcileMessage, setReconcileMessage] = useState<string | null>(null);

  // Shared handler for refreshing projects and reconciling timeline
  const handleRefresh = async () => {
    await refreshProjects();
    
    // If we have a current project, also reconcile its timeline
    if (currentProject) {
      console.log('🔄 Reconciling timeline for current project:', currentProject);
      try {
        const addedCount = await reconcileProjectTimeline(currentProject);
        if (addedCount > 0) {
          setReconcileMessage(`Found and added ${addedCount} orphaned video${addedCount === 1 ? '' : 's'} to timeline`);
          setTimeout(() => setReconcileMessage(null), 5000);
        }
      } catch (reconcileError) {
        console.warn('Timeline reconciliation failed:', reconcileError);
      }
    }
  };

  // Listen for filesystem changes (from Finder, CLI, or other apps)
  useEffect(() => {
    console.log('🎧 Setting up filesystem-changed listener');
    const unlisten = listen('filesystem-changed', async () => {
      console.log('� Filesystem changed event received - refreshing projects list');
      await handleRefresh();
    });

    return () => {
      console.log('🧹 Cleaning up filesystem-changed listener');
      unlisten.then(fn => fn());
    };
  }, [currentProject, reconcileProjectTimeline, refreshProjects]);

  // Listen for clip-saved events (from app recordings)
  useEffect(() => {
    console.log('🎧 Setting up clip-saved listener');
    const unlisten = listen('clip-saved', async () => {
      console.log('📹 Clip saved event received - refreshing projects list');
      await handleRefresh();
    });

    return () => {
      console.log('🧹 Cleaning up clip-saved listener');
      unlisten.then(fn => fn());
    };
  }, [currentProject, reconcileProjectTimeline, refreshProjects]);

  const handleProjectSelect = async (projectName: string) => {
    if (projectName === 'create-new') {
      setIsCreatingProject(true);
      setNewProjectName('');
      setCreateError(null);
      return;
    }

    try {
      await setCurrentProject(projectName);
      
      // Auto-reconcile timeline when project is selected
      try {
        const addedCount = await reconcileProjectTimeline(projectName);
        if (addedCount > 0) {
          setReconcileMessage(`Found and added ${addedCount} orphaned video${addedCount === 1 ? '' : 's'} to timeline`);
          // Clear message after 5 seconds
          setTimeout(() => setReconcileMessage(null), 5000);
        }
      } catch (reconcileError) {
        console.warn('Timeline reconciliation failed:', reconcileError);
        // Don't show error to user for auto-reconcile, just log it
      }
      
      onProjectChange?.(projectName);
    } catch (err) {
      console.error('Failed to set project:', err);
    }
  };

  const handleBrowseFolder = async () => {
    if (!currentProject) return;

    try {
      await invoke('open_folder', { 
        path: `${await invoke<string>('get_output_folder')}/${currentProject}` 
      });
    } catch (err) {
      console.error('Failed to open project folder:', err);
      setReconcileMessage(`❌ Failed to open folder: ${err}`);
      setTimeout(() => setReconcileMessage(null), 5000);
    }
  };

  const handleCreateProject = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!newProjectName.trim()) {
      setCreateError('Project name is required');
      return;
    }

    try {
      setCreateError(null);
      await createProject(newProjectName.trim());
      setIsCreatingProject(false);
      setNewProjectName('');
      onProjectChange?.(newProjectName.trim());
    } catch (err) {
      setCreateError(err as string);
    }
  };

  const handleCancelCreate = () => {
    setIsCreatingProject(false);
    setNewProjectName('');
    setCreateError(null);
  };





  if (loading) {
    return (
      <div className="project-panel">
        <div className="project-panel-header">
          <label>📁 Project</label>
        </div>
        <div className="project-panel-loading">Loading projects...</div>
      </div>
    );
  }

  return (
    <div className="project-panel">
      {!isCreatingProject ? (
        <div className="project-controls">
          <label htmlFor="project-select" className="project-label">Project:</label>
          <div className="clip-counter" aria-live="polite" title={`${clipCount} video${clipCount === 1 ? '' : 's'} in current project`}>
            {clipCount}
          </div>
          <select
            id="project-select"
            value={currentProject || ''}
            onChange={(e) => handleProjectSelect(e.target.value)}
            className="project-select"
            aria-label="Select project"
          >
            <option value="" disabled>
              {projects.length === 0 ? 'No projects found' : 'Select a project'}
            </option>
            
            {projects.map((project) => (
              <option key={project.name} value={project.name}>
                {project.name} ({project.videoCount} videos)
              </option>
            ))}
            
            <option value="create-new" className="create-new-option">
              ➕ Create New Project
            </option>
          </select>

          <button
            onClick={handleBrowseFolder}
            disabled={!currentProject}
            className="action-btn browse-btn"
            title="Open project folder in Finder"
            aria-label="Browse project folder"
          >
            📁
          </button>

          <button
            onClick={() => {
              console.log('🔄 Manual refresh triggered');
              handleRefresh();
            }}
            className="action-btn refresh-btn"
            title="Refresh project list"
            aria-label="Refresh projects"
          >
            🔄
          </button>

          {error && (
            <div className="project-error" role="alert">
              {error}
            </div>
          )}
        </div>
      ) : (
        <form onSubmit={handleCreateProject} className="create-project-form">
          <div className="create-project-input-group">
            <input
              type="text"
              value={newProjectName}
              onChange={(e) => setNewProjectName(e.target.value)}
              placeholder="Enter project name..."
              className="create-project-input"
              autoFocus
              maxLength={50}
              aria-label="New project name"
            />
            <div className="create-project-actions">
              <button 
                type="submit" 
                className="create-project-btn create"
                disabled={!newProjectName.trim()}
                aria-label="Create project"
              >
                ✓
              </button>
              <button 
                type="button" 
                onClick={handleCancelCreate}
                className="create-project-btn cancel"
                aria-label="Cancel create project"
              >
                ✕
              </button>
            </div>
          </div>
          
          {createError && (
            <div className="create-project-error" role="alert">
              {createError}
            </div>
          )}
        </form>
      )}

      {reconcileMessage && (
        <div 
          className={`reconcile-message ${
            reconcileMessage.includes('❌') ? 'reconcile-message--error' : 'reconcile-message--success'
          }`} 
          role="status" 
          aria-live="polite"
        >
          {reconcileMessage}
        </div>
      )}

      <ClipsList 
        projectName={currentProject} 
        outputFolder={outputFolder}
        debugDragDrop={debugDragDrop}
        debugAriaFocus={debugAriaFocus}
      />
    </div>
  );
}