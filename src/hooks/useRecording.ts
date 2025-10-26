import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { RecordingState, ClipSavedEvent, ErrorEvent } from "../types";

/**
 * Hook to manage recording state and listen to backend events
 */
export function useRecording() {
  const [recordingState, setRecordingState] = useState<RecordingState>({
    status: "idle",
    clipCount: 0,
  });

  // Load existing clip count on startup
  useEffect(() => {
    const loadExistingClipCount = async () => {
      try {
        console.log("🔄 Loading existing recordings on startup...");
        const recordings = await invoke<
          Array<{
            filename: string;
            path: string;
            size: number;
            created: number;
          }>
        >("list_recordings");
        console.log(
          `📊 Frontend: Found ${recordings.length} existing recordings:`,
          recordings
        );
        setRecordingState((prev) => ({
          ...prev,
          clipCount: recordings.length,
        }));
      } catch (error) {
        console.error("❌ Failed to load existing recordings:", error);
      }
    };

    loadExistingClipCount();
  }, []);

  useEffect(() => {
    console.log("🎧 Setting up event listeners...");

    // Listen for status updates from Rust backend
    const unlistenStatus = listen<string>("recording-status", (event) => {
      console.log("🎯 Frontend received status:", event.payload);
      setRecordingState((prev) => ({
        ...prev,
        status: event.payload as RecordingState["status"],
      }));
    });

    // Listen for clip saved events
    const unlistenClipSaved = listen<ClipSavedEvent>("clip-saved", (event) => {
      console.log("💾 Frontend received clip-saved:", event.payload);
      setRecordingState((prev) => ({
        ...prev,
        // Don't change status - it's already set to idle by the recording-status event
        clipCount: prev.clipCount + 1,
        currentClipPath: event.payload.path,
      }));
    });

    // Listen for error events
    const unlistenError = listen<ErrorEvent>("recording-error", (event) => {
      setRecordingState((prev) => ({
        ...prev,
        status: "error",
        error: event.payload.message,
      }));
    });

    // Listen for recording deletions to update clip count
    const handleRecordingDeleted = () => {
      // Reload the recordings to get updated count
      const updateClipCount = async () => {
        try {
          const recordings = await invoke<
            Array<{
              filename: string;
              path: string;
              size: number;
              created: number;
            }>
          >("list_recordings");
          setRecordingState((prev) => ({
            ...prev,
            clipCount: recordings.length,
          }));
        } catch (error) {
          console.error("Failed to update clip count after deletion:", error);
        }
      };
      updateClipCount();
    };

    // Listen for custom deletion event
    window.addEventListener("recording-deleted", handleRecordingDeleted);

    console.log("✅ Event listeners set up");

    // Cleanup listeners
    return () => {
      console.log("🧹 Cleaning up event listeners...");
      unlistenStatus.then((fn) => fn());
      unlistenClipSaved.then((fn) => fn());
      unlistenError.then((fn) => fn());
      window.removeEventListener("recording-deleted", handleRecordingDeleted);
    };
  }, []);

  return recordingState;
}
