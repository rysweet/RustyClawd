# Mouse Interaction Requirements

**Version:** 1.0
**Date:** 2025-12-10
**Status:** Draft - Awaiting Review

## Overview

Implement comprehensive mouse interaction support for the RustyClawd TUI application, enabling users to interact with UI components through click events. The system should provide a flexible, extensible architecture for routing mouse events to appropriate handlers.

## Current State

### Existing Mouse Support
- **Location:** `crates/cli/src/tui/event.rs:48-66`
- **Current Implementation:**
  - Mouse scroll events (ScrollUp/ScrollDown) for message history
  - Smooth scrolling (3 lines per scroll)
  - All other mouse events ignored

### Application Structure
```
┌─────────────────────────────────────────────┐
│ Status Bar (not a pane)                     │
├─────────────────────────────────────────────┤
│                                             │
│ Messages Pane                               │
│                                             │
├─────────────────────────────────────────────┤
│ Input Pane (TextArea widget)               │
└─────────────────────────────────────────────┘

With Debug Panel:
┌─────────────────────────────────┬───────────┐
│ Status Bar                      │           │
├─────────────────────────────────┤  Debug    │
│                                 │  Pane     │
│ Messages Pane                   │           │
│                                 │           │
├─────────────────────────────────┤           │
│ Input Pane                      │           │
└─────────────────────────────────┴───────────┘
```

### Key Files
- **`crates/cli/src/tui/app.rs`** - Application state, including focus state
- **`crates/cli/src/tui/event.rs`** - Event handling (keyboard + mouse)
- **`crates/cli/src/tui/ui.rs`** - Rendering layer (pure functions)
- **`crates/cli/src/tui/layout.rs`** - Dynamic layout calculation

## Goals

### Primary Goals
1. **Pane Focus System** - Click any pane to focus it (only one active at a time)
2. **Event Routing Architecture** - Flexible system for routing clicks to appropriate handlers
3. **Visual Feedback** - Clear indication of which pane is focused
4. **Maintainability** - Clean separation between layout, state, and event handling

### Secondary Goals
1. **Status Bar Widgets** - Clickable elements in status bar (future: drop-down menus)
2. **Widget-Level Routing** - Route clicks to specific widgets within panes
3. **Extensibility** - Easy to add new clickable components

## Architecture Design

### 1. Pane System

#### Pane Definition
A **pane** is a major UI region that:
- Can receive focus (only one pane focused at a time)
- Contains one or more widgets
- Has a defined screen area (Rect)
- Has visual indication when focused (e.g., border style/color)

#### Pane Types
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Messages,  // Message history display
    Input,     // Input text area
    Debug,     // Debug panel (when visible)
}
```

**Not Panes:**
- Status bar (contains widgets but not focusable as a pane)
- Modals (autocomplete, memory modal) - overlay behavior

#### Focus State
```rust
// In App struct (crates/cli/src/tui/app.rs)
pub struct App {
    // ... existing fields ...
    focused_pane: Pane,  // NEW: Currently focused pane
}
```

**Default Focus:** Input pane (where user types)

### 2. Hit Testing System

#### Concept
Hit testing determines which UI component was clicked based on mouse coordinates.

```rust
/// Maps screen coordinates to UI components
pub struct HitTestResult {
    pub pane: Option<Pane>,
    pub widget: Option<WidgetId>,  // Future: specific widget within pane
}

impl App {
    /// Perform hit test at mouse coordinates
    pub fn hit_test(&self, x: u16, y: u16, layout: &LayoutAreas) -> HitTestResult {
        // Check each pane's bounding rect
        // Return which pane (if any) contains the coordinates
    }
}
```

#### Implementation Location
- **File:** `crates/cli/src/tui/app.rs`
- **Rationale:** Hit testing requires knowledge of layout areas, but decision logic belongs with app state

### 3. Event Routing

#### Mouse Event Flow
```
1. Mouse click event received (crossterm)
   ↓
2. Extract coordinates (x, y)
   ↓
3. Perform hit test (which pane/widget?)
   ↓
4. Route to appropriate handler
   ↓
5. Update app state (e.g., focused_pane)
   ↓
6. Mark dirty for re-render
```

#### Event Handler Structure
```rust
// In crates/cli/src/tui/event.rs
fn handle_mouse_event(app: &mut App, mouse: event::MouseEvent, layout: &LayoutAreas) -> Result<EventResult> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Perform hit test
            let hit = app.hit_test(mouse.column, mouse.row, layout);

            if let Some(pane) = hit.pane {
                app.set_focused_pane(pane);
            }

            Ok(EventResult::Continue)
        }
        MouseEventKind::ScrollUp => {
            // Existing scroll handling
        }
        // ... other mouse events
    }
}
```

**Challenge:** Event handler needs layout information (Rect bounds) but doesn't have access to it currently.

**Solution Options:**
1. Pass layout info through event handler chain
2. Store layout info in App state (updated each render)
3. Recalculate layout in event handler (less efficient)

**Recommendation:** Option 2 - Store layout areas in App state

### 4. Visual Feedback

#### Focus Indication
Focused pane should have distinct visual appearance:

**Option A: Border Style**
- Focused: Bright/bold border color
- Unfocused: Dimmed border color

**Option B: Border Character**
- Focused: Double-line border (`Borders::DOUBLE`)
- Unfocused: Single-line border (`Borders::ALL`)

**Option C: Both**
- Combine color and style changes

**Recommendation:** Option A (color change) - simpler, less visual clutter

#### Color Scheme
```rust
// Focused pane
const FOCUSED_BORDER: Color = RUST_ORANGE; // Existing: Color::Rgb(222, 165, 132)

// Unfocused pane
const UNFOCUSED_BORDER: Color = Color::Rgb(100, 100, 100); // Dimmed gray
```

### 5. Render Integration

#### Rendering Focus State
```rust
// In crates/cli/src/tui/ui.rs
fn render_messages(frame: &mut Frame, area: Rect, app: &App, throbber: char) -> usize {
    let border_color = if app.focused_pane() == Pane::Messages {
        FOCUSED_BORDER
    } else {
        UNFOCUSED_BORDER
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title("Messages");

    // ... rest of rendering
}
```

**Apply to:**
- `render_messages()` - Messages pane
- `render_input()` - Input pane
- `render_debug_panel()` - Debug pane

### 6. Layout State Management

#### Problem
Event handler needs to know pane boundaries (Rects) to perform hit testing, but layout is calculated in render phase.

#### Solution
Store layout areas in App state, updated during each render:

```rust
// In crates/cli/src/tui/app.rs
pub struct App {
    // ... existing fields ...
    focused_pane: Pane,
    layout_cache: Option<LayoutCache>,  // NEW: Cached layout from last render
}

#[derive(Debug, Clone)]
pub struct LayoutCache {
    pub messages_area: Rect,
    pub input_area: Rect,
    pub debug_area: Option<Rect>,
}
```

**Update in render:**
```rust
// In crates/cli/src/tui/ui.rs
pub fn render(frame: &mut Frame, app: &mut App) -> usize {  // NOTE: &mut App
    let layout = LayoutOrganizer::organize(frame.area(), &config);
    let (messages_area, input_area) = LayoutOrganizer::split_main(layout.main, app);

    // Cache layout for event handling
    app.update_layout_cache(LayoutCache {
        messages_area,
        input_area,
        debug_area: layout.debug,
    });

    // ... continue rendering
}
```

**Issue:** This makes render() mutate app state, violating current pure function design.

**Alternative:** Pass layout cache back as return value:
```rust
pub fn render(frame: &mut Frame, app: &App) -> (usize, LayoutCache) {
    // ... render logic ...
    (max_scroll, layout_cache)
}
```

## Implementation Plan

### Phase 1: Foundation (Pane Focus System)

**File Changes:**
1. **`crates/cli/src/tui/app.rs`**
   - Add `Pane` enum
   - Add `focused_pane: Pane` field to App
   - Add `layout_cache: Option<LayoutCache>` field
   - Add `set_focused_pane()` method
   - Add `focused_pane()` getter
   - Add `hit_test()` method
   - Add `update_layout_cache()` method

2. **`crates/cli/src/tui/ui.rs`**
   - Update `render()` to return `(usize, LayoutCache)`
   - Update `render_messages()` to apply focus border color
   - Update `render_input()` to apply focus border color
   - Update `render_debug_panel()` to apply focus border color

3. **`crates/cli/src/tui/event.rs`**
   - Update `handle_mouse_event()` to handle Left button clicks
   - Add hit testing logic
   - Add focus change on pane click

4. **`crates/cli/src/interactive.rs`** (main loop)
   - Update to handle new render() return type
   - Store layout cache in app state after render

### Phase 2: Status Bar Widgets (Future)

**Deferred to future implementation:**
- Clickable permission mode toggle
- Clickable debug toggle
- Drop-down menus

### Phase 3: Widget-Level Routing (Future)

**Deferred to future implementation:**
- Click individual messages to expand/collapse
- Click specific lines in debug panel
- Widget-specific mouse handlers

## Success Criteria

### Must Have (Phase 1)
- ✅ Click on Messages pane → focuses Messages pane (visual border change)
- ✅ Click on Input pane → focuses Input pane (visual border change)
- ✅ Click on Debug pane (when visible) → focuses Debug pane
- ✅ Only one pane focused at a time
- ✅ Default focus: Input pane
- ✅ Focused pane has distinct visual appearance (border color)
- ✅ No regression in existing scroll wheel behavior

### Nice to Have (Future)
- Click permission mode in status bar to cycle modes
- Click debug indicator to toggle debug panel
- Click message to interact with specific message
- Hover effects (requires MouseEventKind::Moved)

## Edge Cases & Considerations

### 1. Modal Overlays
**Issue:** What happens when autocomplete/memory modal is active?

**Solution:** Modals should capture all mouse events in their area, preventing pane focus changes underneath.

### 2. Streaming State
**Issue:** Should pane focus change during streaming response?

**Solution:** Allow focus changes during streaming (user might want to scroll messages while streaming to input pane).

### 3. Click on Border
**Issue:** Border is part of widget area - should clicking border focus the pane?

**Solution:** Yes, treat entire pane area (including border) as clickable.

### 4. Terminal Resize
**Issue:** Layout cache becomes stale after terminal resize.

**Solution:** Layout cache is updated every render cycle, so resize is automatically handled.

### 5. Multiple Rapid Clicks
**Issue:** User double-clicks or rapidly clicks.

**Solution:** Each click event is processed independently. Rapid clicks to same pane are harmless (redundant focus set).

## Open Questions

1. **Should input pane always auto-focus when user types?**
   - Current: Yes (keyboard events always go to input)
   - Proposed: Keep current behavior (typing auto-focuses input)

2. **Should mouse scroll events respect focused pane?**
   - Current: Scroll always affects messages area
   - Proposed Option A: Scroll affects focused pane
   - Proposed Option B: Keep current behavior (scroll always messages)
   - **Recommendation:** Option B (keep current - less surprising)

3. **What if user clicks between panes (on separator)?**
   - **Recommendation:** No focus change (only clicks within pane areas)

4. **Should we support keyboard shortcuts for pane focus?**
   - Example: F2/F3/F4 to cycle panes
   - **Recommendation:** Yes, add as secondary feature (Tab to cycle?)

## Testing Strategy

### Manual Testing
1. **Click each pane** → verify border color changes
2. **Click pane, then type** → verify input still works
3. **Click pane, then scroll** → verify scroll behavior unchanged
4. **Rapid clicks** → verify no crashes or odd behavior
5. **Click during streaming** → verify focus changes work
6. **Resize terminal, then click** → verify hit testing still accurate
7. **Click with debug panel visible** → verify all 3 panes clickable
8. **Click with debug panel hidden** → verify only 2 panes clickable

### Automated Testing
```rust
#[test]
fn test_hit_test_messages_pane() {
    let mut app = App::new(PermissionMode::default());
    let layout = /* create test layout */;
    app.update_layout_cache(layout);

    let hit = app.hit_test(10, 5, &layout);
    assert_eq!(hit.pane, Some(Pane::Messages));
}

#[test]
fn test_focus_change() {
    let mut app = App::new(PermissionMode::default());
    assert_eq!(app.focused_pane(), Pane::Input); // Default

    app.set_focused_pane(Pane::Messages);
    assert_eq!(app.focused_pane(), Pane::Messages);
}
```

## References

### Ratatui Documentation
- Mouse Events: https://ratatui.rs/concepts/event-handling/
- Hit Testing: (community patterns, not built-in)

### Related Code
- **`crates/cli/src/tui/event.rs:48-66`** - Existing mouse scroll handling
- **`crates/cli/src/tui/layout.rs`** - Layout calculation
- **`crates/cli/src/tui/app.rs`** - Application state

## Review Notes

**For Reviewer:**
- Is the architecture sound for ratatui best practices?
- Any issues with storing layout cache in app state?
- Better approach for pure-function render() vs needing layout cache?
- Any edge cases we haven't considered?
- Is the Pane enum approach idiomatic?
