import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import "./RecordingsList.css";

interface RecordingInfo {
  filename: string;
  path: string;
  size: number;
  created: number;
  thumbnail_path?: string;
}

interface RecordingsListProps {
  onRecordingSelect?: (recording: RecordingInfo) => void;
  onRecordingDeleted?: () => void;
}

// Sortable Recording Item Component
function SortableRecordingItem({
  recording,
  thumbnailDataUrls,
  onRecordingSelect,
  onDelete,
  formatFileSize,
  formatDate,
}: {
  recording: RecordingInfo;
  thumbnailDataUrls: Record<string, string>;
  onRecordingSelect?: (recording: RecordingInfo) => void;
  onDelete: () => void;
  formatFileSize: (bytes: number) => string;
  formatDate: (timestamp: number) => string;
}) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: recording.path });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`recording-item ${isDragging ? "dragging" : ""}`}
      onClick={() => onRecordingSelect?.(recording)}
    >
      <div className="recording-thumbnail">
        {recording.thumbnail_path && thumbnailDataUrls[recording.path] ? (
          <img
            src={thumbnailDataUrls[recording.path]}
            alt="Thumbnail"
            className="thumbnail-image"
            onError={() => {
              console.error(
                "Failed to load thumbnail:",
                recording.thumbnail_path
              );
            }}
          />
        ) : (
          <div className="thumbnail-placeholder">
            <div className="thumbnail-loading">📹</div>
          </div>
        )}
      </div>
      <div className="recording-info">
        <div className="recording-filename">{recording.filename}</div>
        <div className="recording-meta">
          <span className="recording-size">
            {formatFileSize(recording.size)}
          </span>
          <span className="recording-date">
            {formatDate(recording.created)}
          </span>
        </div>
      </div>
      <div className="recording-actions">
        <button
          className="play-btn"
          onClick={async (e) => {
            e.stopPropagation();
            try {
              await invoke("play_recording", { filePath: recording.path });
              console.log("🎬 Playing recording:", recording.filename);
            } catch (error) {
              console.error("Failed to play recording:", error);
            }
          }}
          title="Play recording"
        >
          ▶️
        </button>
        <button
          className="delete-btn"
          onClick={async (e) => {
            e.stopPropagation();
            try {
              await invoke("delete_recording", {
                filename: recording.filename,
              });
              console.log("🗑️ Deleted recording:", recording.filename);
              // Call the parent's onDelete to refresh UI
              onDelete();
            } catch (error) {
              console.error("Failed to delete recording:", error);
            }
          }}
          title="Delete recording"
        >
          🗑️
        </button>
      </div>
      <div
        className="drag-handle"
        {...attributes}
        {...listeners}
        onClick={(e) => e.stopPropagation()}
        onMouseDown={(e) => e.stopPropagation()}
        title="Drag to reorder"
      >
        ⋮⋮
      </div>
    </div>
  );
}

// Convert file path to data URL for display using Tauri command
const getThumbnailDataUrl = async (filePath: string): Promise<string> => {
  try {
    const base64Data = await invoke<string>("read_thumbnail_file", {
      filePath,
    });
    const dataUrl = `data:image/jpeg;base64,${base64Data}`;
    return dataUrl;
  } catch (error) {
    console.error("Failed to load thumbnail:", error);
    return "";
  }
};

export function RecordingsList({
  onRecordingSelect,
  onRecordingDeleted,
}: RecordingsListProps) {
  const [recordings, setRecordings] = useState<RecordingInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [thumbnailDataUrls, setThumbnailDataUrls] = useState<
    Record<string, string>
  >({});

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  const loadRecordings = async () => {
    try {
      setLoading(true);
      console.log("🔄 RecordingsList: Loading recordings...");
      const recordings = await invoke<RecordingInfo[]>("list_recordings");
      console.log("📋 RecordingsList: Received recordings:", recordings);
      setRecordings(recordings);

      // Load thumbnail data URLs
      const thumbnailUrls: Record<string, string> = {};
      for (const recording of recordings) {
        if (recording.thumbnail_path) {
          try {
            const dataUrl = await getThumbnailDataUrl(recording.thumbnail_path);
            if (dataUrl) {
              thumbnailUrls[recording.path] = dataUrl;
            }
          } catch (error) {
            console.error(
              `Failed to load thumbnail for ${recording.filename}:`,
              error
            );
          }
        }
      }
      setThumbnailDataUrls(thumbnailUrls);

      // If any thumbnails failed to load, retry after a delay
      const failedThumbnails = recordings.filter(
        (r) => r.thumbnail_path && !thumbnailUrls[r.path]
      );
      if (failedThumbnails.length > 0) {
        setTimeout(async () => {
          const retryUrls: Record<string, string> = {};
          for (const recording of failedThumbnails) {
            if (recording.thumbnail_path) {
              try {
                const dataUrl = await getThumbnailDataUrl(
                  recording.thumbnail_path
                );
                if (dataUrl) {
                  retryUrls[recording.path] = dataUrl;
                }
              } catch (error) {
                console.error(`Retry failed for ${recording.filename}:`, error);
              }
            }
          }
          if (Object.keys(retryUrls).length > 0) {
            setThumbnailDataUrls((prev) => ({ ...prev, ...retryUrls }));
          }
        }, 2000);
      }
    } catch (error) {
      console.error("Failed to load recordings:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Load recordings immediately on component mount
    loadRecordings();
  }, []);

  // Listen for new recordings to refresh the list
  useEffect(() => {
    const unlistenClipSaved = listen("clip-saved", () => {
      // Add a longer delay to ensure thumbnail is fully generated and written to disk
      setTimeout(() => {
        loadRecordings();
      }, 1000);
    });

    return () => {
      unlistenClipSaved.then((fn) => fn());
    };
  }, []);

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  const formatDate = (timestamp: number): string => {
    const date = new Date(timestamp * 1000);
    return (
      date.toLocaleDateString() +
      " " +
      date.toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      })
    );
  };

  const handleOpenFolder = async () => {
    try {
      const outputFolder = await invoke<string>("get_output_folder");
      await invoke("open_folder", { path: outputFolder });
    } catch (error) {
      console.error("Failed to open folder:", error);
    }
  };

  const handleDragStart = (event: any) => {
    console.log("🔄 Drag started:", event.active.id);
  };

  const handleDragEnd = async (event: DragEndEvent) => {
    try {
      const { active, over } = event;
      console.log("🔄 Drag ended:", { active: active.id, over: over?.id });

      if (over && active.id !== over.id) {
        const oldIndex = recordings.findIndex(
          (item) => item.path === active.id
        );
        const newIndex = recordings.findIndex((item) => item.path === over.id);

        if (oldIndex === -1 || newIndex === -1) {
          console.warn("⚠️ Could not find recording indices for reordering");
          return;
        }

        console.log("🔄 Reordering from index", oldIndex, "to", newIndex);

        const newRecordings = arrayMove(recordings, oldIndex, newIndex);
        setRecordings(newRecordings);

        // Send new order to backend
        try {
          const newOrder = newRecordings.map((recording) => recording.filename);
          await invoke("reorder_recordings", { newOrder });
          console.log("✅ Recordings reordered successfully");

          // Refresh the recordings list to get the updated filenames
          await loadRecordings();
        } catch (error) {
          console.error("❌ Failed to reorder recordings:", error);
          // Revert the UI change if backend fails
          await loadRecordings();
        }
      }
    } catch (error) {
      console.error("❌ Error in handleDragEnd:", error);
    }
  };

  // Thumbnails are now generated automatically when recordings are saved

  if (loading) {
    return (
      <div className="recordings-sidebar">
        <div className="recordings-header">
          <h3>📹 Recordings</h3>
        </div>
        <div className="recordings-loading">Loading...</div>
      </div>
    );
  }

  return (
    <div className="recordings-sidebar">
      <div className="recordings-header">
        <h3>📹 Recordings ({recordings.length})</h3>
        <button
          onClick={handleOpenFolder}
          className="open-folder-btn"
          title="Open recordings folder"
        >
          📁
        </button>
      </div>

      <div className="recordings-list">
        {recordings.length === 0 ? (
          <div className="no-recordings">
            <p>No recordings yet</p>
            <p className="hint">Press ⌘+Shift+R to record</p>
          </div>
        ) : (
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
          >
            <SortableContext
              items={recordings.map((r) => r.path)}
              strategy={verticalListSortingStrategy}
            >
              {recordings.map((recording) => (
                <SortableRecordingItem
                  key={recording.path}
                  recording={recording}
                  thumbnailDataUrls={thumbnailDataUrls}
                  onRecordingSelect={onRecordingSelect}
                  onDelete={() => {
                    // Just refresh the recordings list and notify parent
                    loadRecordings();
                    onRecordingDeleted?.();
                  }}
                  formatFileSize={formatFileSize}
                  formatDate={formatDate}
                />
              ))}
            </SortableContext>
          </DndContext>
        )}
      </div>
    </div>
  );
}
