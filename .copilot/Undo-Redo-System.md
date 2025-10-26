# Undo/Redo System Implementation

## Overview
Implemented a comprehensive undo/redo system for the ClipsList component with keyboard shortcuts (⌘Z/⌘Y) and soft-delete functionality.

## Features Implemented

### 1. **useUndoRedo Hook** (`src/hooks/useUndoRedo.ts`)
A reusable React hook that manages undo/redo history:

- **History Management**: Maintains past, present, and future states
- **Keyboard Shortcuts**: 
  - `⌘Z` or `Ctrl+Z` → Undo
  - `⌘Shift+Z` or `⌘Y` → Redo
- **History Limiting**: Max 50 states (configurable)
- **Checkpoint Control**: Optional checkpoints for granular control
- **API**:
  ```typescript
  const { 
    state,           // Current state
    setState,        // Update state (checkpoint = true by default)
    undo,            // Undo last action
    redo,            // Redo undone action
    canUndo,         // Boolean: can undo?
    canRedo,         // Boolean: can redo?
    clearHistory     // Clear all history
  } = useUndoRedo(initialState);
  ```

### 2. **Soft Delete System**
Clips are never hard-deleted from the timeline:

- **TimelineEntry Extension**:
  ```typescript
  interface TimelineEntry {
    // ... existing fields
    deleted?: boolean;      // Soft delete flag
    deletedAt?: string;     // ISO timestamp of deletion
  }
  ```

- **Behavior**:
  - Deleted clips remain in timeline but are filtered from display
  - Undo restore works by toggling `deleted` flag
  - Backend command `save_project_timeline` persists all clips (including deleted)

### 3. **Backend Commands**

#### `save_project_timeline` (New)
```rust
#[tauri::command]
pub async fn save_project_timeline(
    project_name: String, 
    timeline: ProjectTimeline, 
    state: State<'_, AppState>
) -> Result<(), String>
```

**Purpose**: Save the entire timeline (including reordered clips, label edits, soft deletes) to `timeline.json`

**Features**:
- Auto-updates `lastModified` timestamp
- Pretty-prints JSON for readability
- Atomic file write

### 4. **ClipsList Integration**

#### State Management
- Uses `useUndoRedo` hook for clips array
- Each action (delete, reorder, label edit) creates undo checkpoint
- Undo/redo buttons show availability state

#### Operations That Create Undo Checkpoints
1. **Drag-Drop Reordering**: Checkpoint on drag end
2. **Label Editing**: Checkpoint on save
3. **Soft Delete**: Checkpoint immediately

#### Operations That Don't Create Checkpoints
- Initial load from backend
- Interim drag updates (only final position creates checkpoint)

### 5. **UI Components**

#### Undo/Redo Buttons
Located in `.clips-header`:
```tsx
<button 
  onClick={undo} 
  disabled={!canUndo}
  title="Undo (⌘Z)"
>
  ↶
</button>
<button 
  onClick={redo} 
  disabled={!canRedo}
  title="Redo (⌘Y)"
>
  ↷
</button>
```

**Styling**:
- Disabled state: 30% opacity
- Hover effect on enabled buttons
- Min-width 32px for consistent sizing

## User Workflows

### Delete → Undo Flow
1. User clicks delete button
2. Confirmation dialog appears
3. On confirm:
   - Clip marked `deleted: true` with timestamp
   - Filtered from visible list
   - Undo checkpoint created
   - Timeline saved to backend
4. User presses ⌘Z:
   - Previous state restored (clip no longer deleted)
   - Clip reappears in list
   - Timeline auto-saved

### Reorder → Undo Flow
1. User drags clip to new position
2. Visual feedback during drag
3. On drop:
   - Clips array reordered in state
   - Undo checkpoint created
   - Timeline saved to backend
4. User presses ⌘Z:
   - Previous order restored
   - Timeline auto-saved

### Label Edit → Undo Flow
1. User clicks label, enters edit mode
2. User types new label, presses Enter
3. On save:
   - Label updated in state
   - Undo checkpoint created
   - Timeline saved to backend
4. User presses ⌘Z:
   - Previous label restored
   - Timeline auto-saved

## Data Flow

```
User Action
    ↓
setClips(newState)  → Creates undo checkpoint
    ↓
saveTimeline()      → Persists to backend
    ↓
invoke('save_project_timeline')
    ↓
timeline.json updated
```

## Backend Persistence

### Timeline Structure
```json
{
  "projectName": "MyProject",
  "createdAt": "2025-01-01T00:00:00Z",
  "lastModified": "2025-01-01T12:30:00Z",
  "entries": [
    {
      "id": "uuid",
      "filename": "recording-1.mp4",
      "label": "User-edited label",
      "deleted": false,
      "recordedAt": "2025-01-01T10:00:00Z",
      "durationMs": 5000,
      ...
    },
    {
      "id": "uuid2",
      "deleted": true,
      "deletedAt": "2025-01-01T12:00:00Z",
      ...
    }
  ],
  "metadata": { ... }
}
```

### Soft Delete Benefits
1. **Undo/Redo**: Can restore deleted clips without re-reading files
2. **Audit Trail**: Preserves deletion timestamps
3. **Safe Operations**: No accidental file deletions
4. **Future Features**: Could add "Show Deleted" view, permanent delete option

## Testing Checklist

- [ ] ⌘Z undoes last delete
- [ ] ⌘Y redoes undone delete
- [ ] ⌘Z undoes label edit
- [ ] ⌘Z undoes reorder
- [ ] Multiple undo/redo cycles work
- [ ] Undo buttons disable when no history
- [ ] Timeline persists across app restarts
- [ ] Soft-deleted clips don't show in list
- [ ] Clip counter reflects only visible clips
- [ ] Keyboard shortcuts work with other apps in focus

## Future Enhancements

### Possible Additions
1. **Undo Stack Visualization**: Show list of recent actions
2. **Permanent Delete**: "Empty Trash" command to actually delete files
3. **Batch Operations**: Select multiple clips, delete/restore together
4. **Undo Scope**: Project-level vs. app-level undo history
5. **Auto-save Indicator**: Show when timeline is saving
6. **Conflict Resolution**: Handle concurrent edits if multiple instances

### Performance Considerations
- History limited to 50 states (prevents memory bloat)
- Soft deletes don't require file I/O during undo
- Timeline saves are atomic (no partial writes)

## Architecture Benefits

### Separation of Concerns
- `useUndoRedo`: Generic undo/redo logic (reusable)
- `ClipsList`: Clips-specific business logic
- Backend: Persistence layer

### Type Safety
- Full TypeScript coverage
- Rust backend validates timeline structure
- No runtime type errors

### Testability
- Hook can be tested independently
- Component actions are discrete functions
- Backend command is unit-testable
