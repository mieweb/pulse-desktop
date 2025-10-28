import { useState } from 'react';
import type { AppSettings } from '../types';
import { useActivity } from './useActivity';

const DEFAULT_OUTPUT_FOLDER = '~/Movies/PushToHold'; // macOS default

/**
 * Hook to manage app settings
 */
export function useSettings() {
  const { updateActivity } = useActivity();
  
  const [settings, setSettings] = useState<AppSettings>({
    outputFolder: DEFAULT_OUTPUT_FOLDER,
    captureMode: 'full',
    aspectRatio: '16:9',
    scaleToPreset: false,
    micEnabled: true,
  });

  const updateSettings = (partial: Partial<AppSettings>) => {
    updateActivity();
    setSettings((prev) => ({ ...prev, ...partial }));
  };

  return { settings, updateSettings };
}
