import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Project, ProjectTimeline } from '../types';
import { useActivity } from './useActivity';

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
  
  // Track if we're currently initializing to prevent duplicate calls (React Strict Mode)
  const initializingRef = useRef(false);
  
  // Activity tracking to keep capturer warm
  const { updateActivity } = useActivity();

  const loadProjects = async () => {
    // Prevent duplicate initialization from React Strict Mode
    if (initializingRef.current) {
      console.log('⏭️  Skipping duplicate loadProjects call (already initializing)');
      return;
    }
    
    initializingRef.current = true;
    console.log('🔄 Starting loadProjects...');
    const startTime = performance.now();
    
    try {
      setLoading(true);
      setError(null);
      
      console.log('📡 Invoking get_projects and get_current_project...');
      const [projectsData, currentProjectData] = await Promise.all([
        invoke<Project[]>('get_projects'),
        invoke<string | null>('get_current_project')
      ]);
      
      const invokeTime = performance.now();
      console.log(`⏱️  Backend calls completed in ${(invokeTime - startTime).toFixed(1)}ms`);
      console.log(`📊 Loaded ${projectsData.length} projects, current: ${currentProjectData || 'none'}`);
      
      setProjects(projectsData);
      
      // If no current project is set but projects exist, auto-select the most recent one
      if (!currentProjectData && projectsData.length > 0) {
        // Projects are already sorted by last_modified (newest first) from backend
        const mostRecentProject = projectsData[0].name;
        console.log('🎯 Auto-selecting most recent project:', mostRecentProject);
        const autoSelectStart = performance.now();
        await invoke('set_current_project', { projectName: mostRecentProject });
        const autoSelectTime = performance.now();
        console.log(`⏱️  Auto-select took ${(autoSelectTime - autoSelectStart).toFixed(1)}ms`);
        setCurrentProjectState(mostRecentProject);
      } else {
        setCurrentProjectState(currentProjectData);
      }
      
      const totalTime = performance.now() - startTime;
      console.log(`✅ loadProjects completed in ${totalTime.toFixed(1)}ms total`);
    } catch (err) {
      const errorTime = performance.now() - startTime;
      console.error(`❌ Failed to load projects after ${errorTime.toFixed(1)}ms:`, err);
      setError(err as string);
    } finally {
      setLoading(false);
      initializingRef.current = false;
    }
  };

  const createProject = async (name: string) => {
    try {
      updateActivity(); // Track user activity
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
      updateActivity(); // Track user activity
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
    updateActivity(); // Track user activity
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
      updateActivity(); // Track user activity
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