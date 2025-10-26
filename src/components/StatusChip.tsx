import type { RecordingStatus } from "../types";
import "./StatusChip.css";

interface StatusChipProps {
  status: RecordingStatus;
}

export function StatusChip({ status }: StatusChipProps) {
  console.log("🎬 StatusChip rendering with status:", status);

  const getStatusDisplay = () => {
    switch (status) {
      case "idle":
        return { text: "Idle", className: "status-idle" };
      case "preparing":
        return { text: "Preparing...", className: "status-preparing" };
      case "recording":
        return { text: "Recording", className: "status-recording" };
      case "saving":
        return { text: "Saving...", className: "status-saving" };
      case "error":
        return { text: "Error", className: "status-error" };
      default:
        return { text: "Unknown", className: "" };
    }
  };

  const { text, className } = getStatusDisplay();

  return (
    <div
      className={`status-chip ${className}`}
      role="status"
      aria-live="polite"
      aria-label={`Recording status: ${text}`}
    >
      <span className="status-indicator" />
      <span className="status-text">{text}</span>
    </div>
  );
}
