import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Project, ProjectTimeline } from '../types';

interface UseProjectsReturn {
  projects: Project[];
  currentProject: string | null;
  loading: boolean;
  error: string | null;
  createProject: (name: string) => Promise<void>;
  setCurrentProject: (name: string) => Promise<void>;
  refreshProjects: () => Promise<void>;
  getProjectTimeline: (name: string) => Promise<ProjectTimeline>;
  reconcileProjectTimeline: (name: string) => Promise<number>;
}

export function useProjects(): UseProjectsReturn {
  const [projects, setProjects] = useState<Project[]>([]);
  const [currentProject, setCurrentProjectState] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadProjects = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const [projectsData, currentProjectData] = await Promise.all([
        invoke<Project[]>('get_projects'),
        invoke<string | null>('get_current_project')
      ]);
      
      setProjects(projectsData);
      
      // If no current project is set but projects exist, auto-select the most recent one
      if (!currentProjectData && projectsData.length > 0) {
        // Projects are already sorted by last_modified (newest first) from backend
        const mostRecentProject = projectsData[0].name;
        console.log('Auto-selecting most recent project:', mostRecentProject);
        await invoke('set_current_project', { projectName: mostRecentProject });
        setCurrentProjectState(mostRecentProject);
      } else {
        setCurrentProjectState(currentProjectData);
      }
    } catch (err) {
      console.error('Failed to load projects:', err);
      setError(err as string);
    } finally {
      setLoading(false);
    }
  };

  const createProject = async (name: string) => {
    try {
      setError(null);
      await invoke('create_project', { projectName: name });
      await loadProjects(); // Refresh the list
    } catch (err) {
      console.error('Failed to create project:', err);
      setError(err as string);
      throw err;
    }
  };

  const setCurrentProject = async (name: string) => {
    try {
      setError(null);
      await invoke('set_current_project', { projectName: name });
      setCurrentProjectState(name);
      // Refresh projects to get updated video counts
      await loadProjects();
    } catch (err) {
      console.error('Failed to set current project:', err);
      setError(err as string);
      throw err;
    }
  };

  const refreshProjects = async () => {
    await loadProjects();
  };

  const getProjectTimeline = async (name: string): Promise<ProjectTimeline> => {
    try {
      const timeline = await invoke<ProjectTimeline>('get_project_timeline', { 
        projectName: name 
      });
      return timeline;
    } catch (err) {
      console.error('Failed to get project timeline:', err);
      throw err;
    }
  };

  const reconcileProjectTimeline = async (name: string): Promise<number> => {
    try {
      setError(null);
      const addedCount = await invoke<number>('reconcile_project_timeline', { 
        projectName: name 
      });
      await loadProjects(); // Refresh to update video counts
      return addedCount;
    } catch (err) {
      console.error('Failed to reconcile project timeline:', err);
      setError(err as string);
      throw err;
    }
  };

  useEffect(() => {
    loadProjects();
  }, []);

  return {
    projects,
    currentProject,
    loading,
    error,
    createProject,
    setCurrentProject,
    refreshProjects,
    getProjectTimeline,
    reconcileProjectTimeline,
  };
}