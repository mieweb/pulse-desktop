import { useState, useEffect } from "react";
import { useRecording } from "./hooks/useRecording";
import { useSettings } from "./hooks/useSettings";
import { StatusChip } from "./components/StatusChip";
import { SettingsPanel } from "./components/SettingsPanel";
import { RegionOverlay } from "./components/RegionOverlay";
import { RecordingsList } from "./components/RecordingsList";
import { invoke } from "@tauri-apps/api/core";
import type { CaptureRegion } from "./types";
import "./App.css";

function App() {
  const recordingState = useRecording();
  const { settings, updateSettings } = useSettings();

  const [isRegionSelectorMode, setIsRegionSelectorMode] = useState(false);
  const [regionSelectorConfig, setRegionSelectorConfig] = useState<{
    aspectRatio: string;
    scaleToPreset: boolean;
  }>({ aspectRatio: "none", scaleToPreset: false });

  // Check if this is the region selector window
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const mode = params.get("mode");

    if (mode === "region-selector") {
      setIsRegionSelectorMode(true);
      setRegionSelectorConfig({
        aspectRatio: params.get("aspectRatio") || "none",
        scaleToPreset: params.get("scaleToPreset") === "true",
      });

      // Set body class for region selector styling
      document.body.classList.add("region-selector-mode");
      document.body.style.background = "transparent";
      document.body.style.margin = "0";
      document.body.style.padding = "0";
      document.body.style.overflow = "hidden";
    }
  }, []);

  const handleOpenFolder = async () => {
    if (recordingState.currentClipPath) {
      try {
        // Extract folder path from file path
        const folderPath = recordingState.currentClipPath.substring(
          0,
          recordingState.currentClipPath.lastIndexOf("/")
        );
        await invoke("open_folder", { path: folderPath });
      } catch (error) {
        console.error("Failed to open folder:", error);
      }
    }
  };

  const handleOpenVideo = async () => {
    if (recordingState.currentClipPath) {
      try {
        await invoke("open_file", { path: recordingState.currentClipPath });
      } catch (error) {
        console.error("Failed to open video:", error);
      }
    }
  };

  const handleRegionSelected = async (region: CaptureRegion) => {
    console.log("handleRegionSelected called with:", region);
    console.log("isRegionSelectorMode:", isRegionSelectorMode);

    try {
      await invoke("set_capture_region", {
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
      });

      if (isRegionSelectorMode) {
        console.log("Closing region selector window...");
        await invoke("close_region_selector");
        console.log("Window close command sent");
      }

      console.log("Region set:", region);
    } catch (error) {
      console.error("Failed to set capture region:", error);
    }
  };

  const handleRegionCancel = async () => {
    console.log("handleRegionCancel called");
    console.log("isRegionSelectorMode:", isRegionSelectorMode);

    if (isRegionSelectorMode) {
      // Close the region selector window and clear region
      try {
        console.log("Clearing region and closing window...");
        await invoke("clear_capture_region");
        await invoke("close_region_selector");
        console.log("Cancel window close command sent");
      } catch (error) {
        console.error("Failed to cancel region selection:", error);
      }
    } else {
      // If user cancels, clear any existing region and return to full screen
      if (settings.captureMode === "region") {
        updateSettings({ captureMode: "full" });
        try {
          await invoke("clear_capture_region");
        } catch (error) {
          console.error("Failed to clear capture region:", error);
        }
      }
    }
  };

  const handleRecordingDeleted = async () => {
    // Refresh the recordings list and update clip count
    // Dispatch custom event to update clip count
    window.dispatchEvent(new CustomEvent("recording-deleted"));
  };

  const handleCaptureModeChange = async (mode: "full" | "region") => {
    updateSettings({ captureMode: mode });

    if (mode === "region") {
      // Open region selection as a new window covering the entire screen
      try {
        await invoke("open_region_selector", {
          aspectRatio: settings.aspectRatio,
          scaleToPreset: settings.scaleToPreset,
        });
      } catch (error) {
        console.error("Failed to open region selector:", error);
      }
    } else {
      // Clear region when switching to full screen
      try {
        await invoke("clear_capture_region");
      } catch (error) {
        console.error("Failed to clear capture region:", error);
      }
    }
  };

  // If this is the region selector window, only show the overlay
  if (isRegionSelectorMode) {
    return (
      <RegionOverlay
        isVisible={true}
        aspectRatio={regionSelectorConfig.aspectRatio as any}
        scaleToPreset={regionSelectorConfig.scaleToPreset}
        onRegionSelected={handleRegionSelected}
        onCancel={handleRegionCancel}
      />
    );
  }

  return (
    <div className="app-layout">
      <RecordingsList onRecordingDeleted={handleRecordingDeleted} />
      <main className="main-content">
        <header className="app-header">
          <h1>🎬 Pulse Desktop</h1>
          <StatusChip status={recordingState.status} />
        </header>

        <SettingsPanel
          settings={settings}
          onSettingsChange={updateSettings}
          onCaptureModeChange={handleCaptureModeChange}
          clipCount={recordingState.clipCount}
        />

        {recordingState.error && (
          <div className="error-message" role="alert" aria-live="assertive">
            <strong>Error:</strong> {recordingState.error}
          </div>
        )}

        {recordingState.currentClipPath && (
          <div className="success-message" role="status" aria-live="polite">
            <div className="success-content">
              <span>
                ✅ Saved: {recordingState.currentClipPath.split("/").pop()}
              </span>
              <div className="success-actions">
                <button
                  onClick={handleOpenVideo}
                  className="action-button"
                  aria-label="Open video file"
                >
                  ▶️ Play Video
                </button>
                <button
                  onClick={handleOpenFolder}
                  className="action-button"
                  aria-label="Open containing folder"
                >
                  📁 Open Folder
                </button>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
