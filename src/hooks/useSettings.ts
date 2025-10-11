import { useState } from 'react';
import type { AppSettings } from '../types';

const DEFAULT_OUTPUT_FOLDER = '~/Movies/PushToHold'; // macOS default

/**
 * Hook to manage app settings
 */
export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>({
    outputFolder: DEFAULT_OUTPUT_FOLDER,
    captureMode: 'full',
    aspectRatio: '16:9',
    scaleToPreset: false,
    micEnabled: true,
  });

  const updateSettings = (partial: Partial<AppSettings>) => {
    setSettings((prev) => ({ ...prev, ...partial }));
  };

  return { settings, updateSettings };
}
