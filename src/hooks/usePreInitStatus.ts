import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { PreInitStatus } from '../types';

/**
 * Hook to track the capturer pre-initialization status
 */
export function usePreInitStatus() {
  const [status, setStatus] = useState<PreInitStatus>('NotInitialized');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Get initial status
    const getInitialStatus = async () => {
      try {
        const initialStatus = await invoke<PreInitStatus>('get_pre_init_status');
        setStatus(initialStatus);
      } catch (err) {
        console.error('Failed to get pre-init status:', err);
        setStatus('NotInitialized');
      } finally {
        setLoading(false);
      }
    };

    getInitialStatus();

    // Listen for status changes
    const unlistenStatus = listen<PreInitStatus>('pre-init-status-changed', (event) => {
      console.log('🎯 Pre-init status changed:', event.payload);
      setStatus(event.payload);
    });

    // Listen for idle shutdown events
    const unlistenShutdown = listen('pre-init-idle-shutdown', () => {
      console.log('💤 Pre-init capturer shut down due to idle timeout');
      setStatus('NotInitialized');
    });

    // Cleanup listeners
    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenShutdown.then((fn) => fn());
    };
  }, []);

  return { status, loading };
}