import { useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * Hook for tracking user activity to keep capturer pre-initialization warm
 */
export function useActivity() {
  const updateActivity = useCallback(async () => {
    try {
      await invoke('update_activity');
    } catch (error) {
      // Silently fail - activity tracking is not critical
      console.debug('Failed to update activity:', error);
    }
  }, []);

  return { updateActivity };
}