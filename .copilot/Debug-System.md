# Debug System Implementation

## Overview
Added comprehensive debugging system for drag-drop and ARIA/keyboard accessibility features in ClipsList component, plus enhanced visual drop indicator.

## Recent Enhancement: Drop Indicator (v2)

### Problem
- Original implementation highlighted the target clip when dragging
- Users couldn't clearly see WHERE the clip would be inserted
- Confusing when dragging between clips

### Solution
- Added visual drop indicator line that appears BETWEEN clips
- Shows exact insertion point before dropping
- Animated pulsing accent-colored line
- Clear visual feedback matching modern UI patterns

### Implementation Details

#### State Management
```typescript
// Replaced dragOverClipId with dropIndicatorPosition
const [dropIndicatorPosition, setDropIndicatorPosition] = useState<number | null>(null);
```

#### Drop Position Logic
```typescript
// Calculate where to show the drop indicator
// If dragging down (draggedIdx < targetIndex), show indicator AFTER target
// If dragging up (draggedIdx > targetIndex), show indicator BEFORE target
const dropPosition = draggedIdx < targetIndex ? targetIndex + 1 : targetIndex;
```

#### Visual Structure
```tsx
<div className="clip-wrapper">
  {/* Drop indicator BEFORE clip */}
  {dropIndicatorPosition === index && <DropIndicator />}
  
  <div className="clip-row">...</div>
  
  {/* Drop indicator AFTER clip (for last position) */}
  {dropIndicatorPosition === index + 1 && <DropIndicator />}
</div>
```

#### CSS Styling
```css
.drop-indicator-line {
  height: 3px;
  background: var(--color-accent);
  border-radius: 2px;
  box-shadow: 0 0 8px var(--color-accent);
  animation: pulse-drop 0.8s ease-in-out infinite;
}
```

### Visual Behavior
1. **Start drag**: Dragged clip fades to 50% opacity, slightly smaller
2. **Drag over clip**: Accent-colored line appears between clips at drop position
3. **Line animates**: Pulsing effect draws attention to insertion point
4. **Drop**: Clip moves to position, indicator disappears
5. **Undo available**: Can undo with ⌘Z

## Changes Made

### 1. Debug Controls Component (NEW)
**File:** `src/components/DebugControls.tsx`
- Floating debug panel in bottom-right corner
- Toggle switches for:
  - 🎯 Drag & Drop debugging
  - ⌨️ ARIA & Focus debugging
- Shows console log prefixes guide
- Clean, unobtrusive UI

### 2. Fixed Drag-Drop Bug
**Problem:** Drag-drop was not saving final reordered state
**Root Cause:** `handleDragEnd` was saving the old `clips` state instead of current state
**Solution:** 
```typescript
// BEFORE (WRONG):
const handleDragEnd = async () => {
  setClips(clips, true); // Saving OLD state
  await saveTimeline(clips); // Saving OLD state
};

// AFTER (CORRECT):
const handleDragEnd = async () => {
  // clips variable is NOW the current state after all drag-over updates
  setClips(clips, true); // Create checkpoint with CURRENT state
  await saveTimeline(clips); // Save CURRENT state
};
```

### 3. Enhanced Drag-Drop with Debugging
**File:** `src/components/ClipsList.tsx`

Added verbose logging throughout drag-drop lifecycle:

#### `handleDragStart`
Logs:
- Clip ID being dragged
- Clip label/filename
- Total clips count

#### `handleDragOver`
Logs:
- Dragged clip ID
- Target clip ID (where dragging over)
- Current clip labels
- Index positions (from → to)
- Full clip order array
- Visual feedback via `dragOverClipId` state

#### `handleDragEnd`
Logs:
- Final clip order
- Whether save will occur
- Save status (before/after)

### 4. Keyboard Navigation Debugging
**File:** `src/components/ClipsList.tsx`

Added comprehensive logging for all keyboard interactions:

#### Arrow Key Navigation (↑↓)
Logs:
- Key pressed
- Current clip and index
- Target clip and index
- Direction (up/down)

#### Keyboard Reordering (⌘↑/⌘↓)
Logs:
- Which clip is being moved
- Direction
- Current position

#### Action Keys
Logs:
- Enter: Which clip is being played
- Delete/Backspace: Which clip is being deleted
- Space: Which clip is being focused

### 5. Focus Change Debugging
**File:** `src/components/ClipsList.tsx`

#### `onFocus` handler
Logs:
- Clip ID receiving focus
- Clip label/filename
- Index position
- Total visible clips count

### 6. Visual Drag Feedback
**File:** `src/components/ClipsList.css`

Added new CSS classes:

```css
.clip-row.drag-over {
  border-top: 3px solid var(--color-accent);
  border-bottom: 3px solid var(--color-accent);
  background: var(--color-bg-hover);
  transform: scale(1.02);
}

.clip-row.dragging {
  opacity: 0.5;
  transform: scale(0.98);
  background: var(--color-bg-primary);
}
```

**Visual States:**
- **Being dragged**: Faded (50% opacity), slightly smaller
- **Drag over target**: Accent borders, highlighted background, slightly larger
- **Normal**: Default appearance

## Console Log Format

### Debug Prefixes
```
🎯 [DRAG] - All drag-drop events
⌨️ [KEYBOARD] - All keyboard shortcuts
👁️ [FOCUS] - Focus changes
```

### Example Console Output

#### Drag-Drop Session
```
🎯 [DRAG] Start: {clipId: 'abc123', clipLabel: 'My Video', totalClips: 5}
🎯 [DRAG] Over: {draggedClipId: 'abc123', targetClipId: 'def456', ...}
🎯 [DRAG] Reordering: {from: 0, to: 2, clipOrder: [...]}
🎯 [DRAG] End: {draggedClipId: 'abc123', finalClipOrder: [...], willSave: true}
🎯 [DRAG] Saving timeline to backend...
🎯 [DRAG] Timeline saved successfully
```

#### Keyboard Navigation Session
```
⌨️ [KEYBOARD]: {key: 'ArrowDown', currentClip: 'Video 1', currentIndex: 0}
⌨️ [KEYBOARD] Navigate Down: {from: 0, to: 1, toClip: 'Video 2'}
👁️ [FOCUS] Clip focused: {clipId: 'def456', clipLabel: 'Video 2', index: 1}

⌨️ [KEYBOARD]: {key: 'ArrowDown', metaKey: true, currentClip: 'Video 2'}
⌨️ [KEYBOARD] Move clip down: 'Video 2'
🎯 [DRAG] Saving timeline to backend... (from keyboard reorder)

⌨️ [KEYBOARD]: {key: 'Enter', currentClip: 'Video 2'}
⌨️ [KEYBOARD] Play clip: 'Video 2'
```

## Usage

### Enable Debugging
1. Click the **🐛 Debug** button in bottom-right corner
2. Check the debugging options you want:
   - ✅ Drag & Drop
   - ✅ ARIA & Focus
3. Open browser console (Cmd+Option+I on macOS)
4. Perform actions (drag clips, use keyboard shortcuts)
5. Watch detailed logs in console

### Disable Debugging
1. Click **🐛 Debug** button
2. Uncheck the debugging options
3. Debug logs will stop

### Understanding the Logs

#### When Drag-Drop Fails
Look for:
- ❌ Missing "End" log → drag operation interrupted
- ❌ "willSave: false" → no project selected or dragged ID null
- ❌ Warnings about invalid indices → data mismatch

#### When Keyboard Navigation Fails
Look for:
- ❌ Key pressed but no "Navigate" log → event not captured
- ❌ "Move clip" log without "Saving" log → save failed
- ❌ Focus change without keyboard event → programmatic focus

## Integration

### Component Hierarchy
```
App.tsx
├─ DebugControls (floating panel)
├─ ProjectPanel
│  └─ ClipsList
│     ├─ debugDragDrop (prop)
│     └─ debugAriaFocus (prop)
└─ SettingsPanel
```

### Props Flow
```typescript
// App.tsx
const [debugDragDrop, setDebugDragDrop] = useState(false);
const [debugAriaFocus, setDebugAriaFocus] = useState(false);

// Pass to ProjectPanel
<ProjectPanel 
  debugDragDrop={debugDragDrop}
  debugAriaFocus={debugAriaFocus}
/>

// ProjectPanel passes to ClipsList
<ClipsList 
  debugDragDrop={debugDragDrop}
  debugAriaFocus={debugAriaFocus}
/>

// ClipsList uses as constants
const DEBUG_DRAG_DROP = debugDragDrop;
const DEBUG_ARIA_FOCUS = debugAriaFocus;
```

## Testing

### Test Drag-Drop
1. Enable "Drag & Drop" debugging
2. Open console
3. Drag a clip to new position
4. Verify you see:
   - ✅ "Start" log
   - ✅ Multiple "Over" logs (one per drag-over)
   - ✅ "Reordering" logs showing position changes
   - ✅ "End" log with final order
   - ✅ "Saving" and "saved successfully" logs

### Test Keyboard Navigation
1. Enable "ARIA & Focus" debugging
2. Open console
3. Click on a clip to focus it
4. Press ↓ arrow key
5. Verify you see:
   - ✅ Keyboard event log with key details
   - ✅ "Navigate Down" log with indices
   - ✅ Focus change log with clip details

### Test Keyboard Reordering
1. Enable "ARIA & Focus" debugging
2. Focus a clip
3. Press ⌘↓ (or Ctrl↓ on Windows)
4. Verify you see:
   - ✅ Keyboard event with metaKey: true
   - ✅ "Move clip down" log
   - ✅ Timeline save log

### Test Visual Feedback
1. Enable "Drag & Drop" debugging
2. Start dragging a clip
3. Verify visual states:
   - ✅ Dragged clip: faded, slightly smaller
   - ✅ Drag-over target: accent borders, highlighted
4. Release to complete drag
5. Verify clip moves to new position

## Performance Notes

### Debug Mode Impact
- **Minimal impact** when logging is disabled (default)
- **Slight impact** when enabled due to console.log calls
- Recommend **disabling in production** builds

### Optimization Opportunities
- Could use `console.group()` for nested logs
- Could add log buffering to reduce console calls
- Could add log export feature for bug reports

## Future Enhancements

### Possible Additions
1. **Log Export**: Save logs to file for bug reports
2. **Performance Metrics**: Log timing for drag/keyboard operations
3. **State Snapshots**: Capture full state on each action
4. **Replay System**: Record and replay user interactions
5. **Visual Debugger**: Show drag paths and focus flow in UI
6. **Undo/Redo Logging**: Track all checkpoint creations

### Advanced Features
- Integration with browser DevTools
- Remote debugging support
- Real-time log streaming
- Log filtering and search
