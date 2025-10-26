# ClipsList Component - Timeline View

## Overview
Redesigned from a grid of video previews to a compact, reorderable timeline list with 36px thumbnails.

## Features

### 1. **Compact List Layout**
- Single-row items (not grid)
- 36px tall static PNG thumbnails
- Efficient use of space (25-30 clips visible without scrolling)

### 2. **Drag-and-Drop Reordering**
- Drag handle (⋮⋮) on left side
- Visual feedback during drag (opacity, scale)
- Persists reordered timeline to `timeline.json`

### 3. **Editable Labels**
- Click label to edit inline
- Press Enter to save, Escape to cancel
- Falls back to filename if no label set
- Persists to `timeline.json`

### 4. **Metadata Display**
- Duration (MM:SS format)
- Resolution (e.g., 1920×1080)
- Aspect ratio (16:9, 9:16, none)
- Recorded date/time
- Mic indicator (🎤) if audio enabled

### 5. **Action Buttons**
- **Play** (▶️): Opens clip in system player
- **Delete** (🗑️): Removes clip from timeline and disk
- Buttons fade in on hover

## Structure

```
ClipsList
├── clips-header (clip count)
└── clips-container (scrollable)
    └── clip-row (for each clip)
        ├── clip-drag-handle (⋮⋮)
        ├── clip-thumbnail (64×36px)
        ├── clip-details
        │   ├── clip-label (editable)
        │   └── clip-metadata (duration, resolution, aspect, time, mic)
        └── clip-actions
            ├── Play button
            └── Delete button
```

## Data Flow

### Loading Clips
1. Component receives `projectName` prop
2. Calls `invoke('get_timeline', { projectName })`
3. Sorts by `recordedAt` (newest first)
4. Renders list

### Reordering
1. User drags clip row
2. `handleDragStart` → sets `draggedIndex`
3. `handleDragOver` → reorders clips array in state
4. `handleDragEnd` → calls `invoke('save_timeline')` to persist

### Label Editing
1. User clicks label → enters edit mode
2. `handleLabelEdit` → sets `editingId` and `editLabel`
3. User types, presses Enter or clicks away
4. `handleLabelSave` → updates clips array, calls `invoke('save_timeline')`

### Deleting
1. User clicks delete button → confirmation dialog
2. `handleDelete` → calls `invoke('delete_clip')`
3. Updates local state and notifies parent via `onClipsUpdate`

## Thumbnail Support

### Current State
- Component expects `thumbnail` field in `TimelineEntry` (e.g., `"thumbnail-1.png"`)
- Falls back to placeholder emoji (🎬) if no thumbnail

### Backend Requirements (TODO)
- **Generate PNG thumbnail during recording**
  - Extract frame at 1 second or middle of video
  - Save as `thumbnail-N.png` in project folder
- **Update `timeline.json` entry**
  - Add `thumbnail` field with filename
- **Rust commands needed:**
  - Update `save_clip` command to generate thumbnail
  - Ensure thumbnail is created before writing timeline entry

## CSS Highlights

### Variables Used
- `--color-bg-primary`, `--color-bg-secondary`, `--color-bg-tertiary`: Backgrounds
- `--color-border-primary`, `--color-border-secondary`: Borders
- `--color-text-primary`, `--color-text-secondary`, `--color-text-muted`: Text colors
- `--color-accent`: Accent color (used for focus, active states)

### Key Styles
- `.clip-row`: Main container, `display: flex`, `cursor: move`
- `.clip-thumbnail`: 64×36px fixed size, `object-fit: cover`
- `.clip-actions`: `opacity: 0` by default, shown on hover
- `.dragging`: Visual feedback during drag (opacity 0.5, scale 0.98)

## Integration

### Usage in ProjectPanel
```tsx
<ClipsList 
  projectName={currentProject}
  outputFolder={settings.outputFolder}
  onClipsUpdate={(count) => setClipCount(count)}
/>
```

### Required Tauri Commands
- `get_timeline` - Load timeline entries for project
- `save_timeline` - Save reordered/edited timeline
- `delete_clip` - Delete clip file and timeline entry
- `open_clip` - Open clip in system player

## Future Enhancements (Mentioned by User)
- **Split functionality**: Split clip into multiple segments
- **Trim functionality**: Trim start/end of clip
- **Timeline markers**: Visual indicators for splits/edits
- **Bulk operations**: Select multiple clips for batch actions

## Design Philosophy
- **Timeline-first**: Linear list better represents video editing workflow than grid
- **Quick navigation**: Small thumbnails allow seeing many clips at once
- **Immediate feedback**: Drag-drop and inline editing feel instant
- **Professional UX**: Mimics video editing software (Final Cut, Premiere)
