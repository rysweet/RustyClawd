# 🔌 rat-focus Integration Architecture for RustyClawd TUI

**Status:** Comprehensive Design Document
**Target:** Phase 1-3 Implementation Roadmap
**Philosophy:** Ruthless simplicity meets robust focus management
**Date:** 2025-12-10

---

## 🎯 Executive Summary

The current MOUSE_INTERACTION_REQUIREMENTS.md proposes a custom hit-testing and focus system. **rat-focus provides a battle-tested, idiomatic solution** that handles:
- Focus state management via `FocusFlag`
- Z-ordering for overlays (autocomplete, modals)
- Mouse event routing with area-based hit testing
- Tab/BackTab navigation out of the box
- Visual feedback patterns

**Key Decision:** Replace custom pane focus system with rat-focus architecture while preserving the clean state/render separation.

---

## 📊 Current Architecture Analysis

### What We Have
```
App State (app.rs)
├── messages: Vec<Message>
├── input: TextArea<'static>        ← Stateful widget from tui-textarea
├── scroll_offset: usize
├── autocomplete: Option<AutocompleteState>
├── memory_modal: Option<MemoryModalState>
├── debug_visible: bool
└── streaming: Option<StreamingState>

Event Handling (event.rs)
├── handle_key_event()
├── handle_mouse_event()           ← Only scroll events
└── Mouse clicks ignored

Rendering (ui.rs) - Pure Functions
├── render()                       ← Main entry, pure fn
├── render_messages()
├── render_input()                 ← Renders TextArea widget
├── render_autocomplete()          ← Overlay popup
├── render_memory_modal()          ← Overlay popup
└── render_debug_panel()
```

### Key Observations

1. **TextArea is already stateful** - `tui-textarea::TextArea` manages its own state
2. **Pure render functions** - `render()` takes `&App`, no mutation
3. **Manual z-ordering** - `Clear` widget used before overlays (lines 559, 605 in ui.rs)
4. **No focus concept** - Everything implicitly focused on input pane
5. **Layout cache proposed** - MOUSE_INTERACTION_REQUIREMENTS wants to store Rects in App

---

## 🧬 rat-focus Core Concepts Mapping

### 1. HasFocus Trait

**What it is:** Interface for focusable components. Widgets implement this to participate in focus management.

**Key methods:**
```rust
trait HasFocus {
    fn build(&self, builder: &mut FocusBuilder);  // Register with focus system
    fn focus(&self) -> FocusFlag;                  // Return focus state
    fn area(&self) -> Rect;                        // Bounding box for hit testing
    fn area_z(&self) -> Option<(Rect, u16)> { None }  // Z-ordering for overlays
    fn navigable(&self) -> Navigation { Regular }   // Tab navigation behavior
}
```

**RustyClawd Mapping:**
- Messages pane → implements HasFocus
- Input pane (TextArea) → wrapper struct implements HasFocus
- Debug pane → implements HasFocus
- Autocomplete popup → implements HasFocus with z-ordering
- Memory modal → implements HasFocus with higher z-order

### 2. FocusFlag

**What it is:** Lightweight focus state holder using `Rc` internally for cheap cloning.

**Key methods:**
```rust
impl FocusFlag {
    fn new() -> Self;
    fn is_focused(&self) -> bool;
    fn gained(&self) -> bool;  // Focus just gained this cycle
    fn lost(&self) -> bool;    // Focus just lost this cycle
}
```

**RustyClawd Usage:**
```rust
// In App state
pub struct App {
    // Existing fields...

    // NEW: Focus state for each focusable component
    focus_messages: FocusFlag,
    focus_input: FocusFlag,
    focus_debug: FocusFlag,
    focus_autocomplete: FocusFlag,
    focus_memory_modal: FocusFlag,
}
```

### 3. FocusBuilder & Focus

**What they are:**
- `FocusBuilder` - Collects focusable widgets in order
- `Focus` - Navigation controller (next/prev/focus_at)

**Usage Pattern (from docs):**
```rust
// Rebuild focus structure each event cycle
let mut builder = FocusBuilder::new();
builder.widget(&messages_pane);
builder.widget(&input_pane);
if debug_visible {
    builder.widget(&debug_pane);
}
let focus = builder.build();

// Handle event
focus.handle(event, Regular);
```

**Critical Insight:** The focus structure is **rebuilt every event**, not persisted. This is perfect for RustyClawd's dynamic layout (debug panel appears/disappears).

### 4. Z-Ordering System

**How it works:**
- Default z-level: 0
- Overlays specify `area_z()` with higher values
- Mouse clicks check z-order, highest wins
- Perfect for autocomplete (z=1) and memory modal (z=2)

**Current Manual Z-Ordering:**
```rust
// ui.rs:559 - Autocomplete overlay
frame.render_widget(Clear, popup_area);  // Manual clear
frame.render_widget(list, popup_area);

// ui.rs:605 - Memory modal overlay
frame.render_widget(Clear, popup_area);  // Manual clear
frame.render_widget(list, popup_area);
```

**With rat-focus:**
```rust
impl HasFocus for AutocompletePopup {
    fn area_z(&self) -> Option<(Rect, u16)> {
        Some((self.popup_area, 1))  // Z-order 1
    }
}

impl HasFocus for MemoryModal {
    fn area_z(&self) -> Option<(Rect, u16)> {
        Some((self.popup_area, 2))  // Z-order 2 (higher priority)
    }
}
```

---

## 🏗️ Proposed Architecture

### Component Hierarchy

```
FocusRoot (rebuilt each event)
├── MessagesPane (z=0)
│   ├── area: messages_area Rect
│   └── focus_flag: app.focus_messages
├── InputPane (z=0)
│   ├── area: input_area Rect
│   ├── focus_flag: app.focus_input
│   └── textarea: TextArea (nested state)
├── DebugPane (z=0, conditional)
│   ├── area: debug_area Rect
│   └── focus_flag: app.focus_debug
├── AutocompletePopup (z=1, conditional)
│   ├── area: popup_area Rect
│   └── focus_flag: app.focus_autocomplete
└── MemoryModal (z=2, conditional)
    ├── area: modal_area Rect
    └── focus_flag: app.focus_memory_modal
```

**Navigation Order:** Messages → Input → Debug (if visible) → skip overlays (not Tab-navigable)

**Mouse Focus:** All components clickable, z-order determines winner for overlapping areas

### State Structure Changes

```rust
// crates/cli/src/tui/app.rs

use rat_focus::{FocusFlag, Focus, FocusBuilder, HasFocus};

pub struct App {
    // Existing fields preserved...
    messages: Vec<Message>,
    input: TextArea<'static>,
    scroll_offset: usize,
    autocomplete: Option<AutocompleteState>,
    memory_modal: Option<MemoryModalState>,

    // NEW: Focus state (replaces proposed focused_pane enum)
    focus_messages: FocusFlag,
    focus_input: FocusFlag,
    focus_debug: FocusFlag,
    focus_autocomplete: FocusFlag,
    focus_memory_modal: FocusFlag,

    // NEW: Layout cache (still needed for area() calculations)
    layout_cache: Option<LayoutCache>,
}

// Helper structs for HasFocus implementations
pub struct MessagesPaneWrapper<'a> {
    app: &'a App,
    area: Rect,
}

pub struct InputPaneWrapper<'a> {
    app: &'a App,
    area: Rect,
}

// Similar for debug, autocomplete, memory modal
```

### Event Handling Integration

```rust
// crates/cli/src/tui/event.rs

pub fn handle_event(app: &mut App, event: Event) -> Result<EventResult> {
    // Build focus structure from current app state
    let mut builder = FocusBuilder::new();

    if let Some(cache) = &app.layout_cache {
        // Register focusable components in Tab order
        builder.widget(&MessagesPaneWrapper {
            app,
            area: cache.messages_area
        });
        builder.widget(&InputPaneWrapper {
            app,
            area: cache.input_area
        });
        if let Some(debug_area) = cache.debug_area {
            builder.widget(&DebugPaneWrapper {
                app,
                area: debug_area
            });
        }

        // Overlays (not Tab-navigable, only mouse-focusable)
        if app.autocomplete_active() {
            builder.widget(&AutocompletePopupWrapper {
                app,
                area: /* calculate from input_area */
            });
        }
        if app.memory_modal_active() {
            builder.widget(&MemoryModalWrapper {
                app,
                area: /* calculate from input_area */
            });
        }
    }

    let focus = builder.build();

    // Let rat-focus handle event (Tab, mouse clicks)
    let outcome = focus.handle(&event, Regular)?;

    if outcome == Outcome::Changed {
        app.mark_dirty();
    }

    // Then handle component-specific events
    match event {
        Event::Key(key) => handle_key_event(app, key),
        Event::Mouse(mouse) => handle_mouse_event(app, mouse),
        Event::Resize(_, _) => Ok(EventResult::Continue),
        _ => Ok(EventResult::Continue),
    }
}
```

### Rendering Integration

**Current render signature:**
```rust
pub fn render(frame: &mut Frame, app: &App) -> usize
```

**Problem:** We need to update `layout_cache` in App, but render takes `&App` (immutable).

**Solution: Return layout cache**
```rust
pub fn render(frame: &mut Frame, app: &App) -> (usize, LayoutCache) {
    let layout = LayoutOrganizer::organize(frame.area(), &config);
    let (messages_area, input_area) = LayoutOrganizer::split_main(layout.main, app);

    let cache = LayoutCache {
        messages_area,
        input_area,
        debug_area: layout.debug,
    };

    // Render with focus-aware styling
    render_messages(frame, messages_area, app);
    render_input(frame, input_area, app);

    (max_scroll, cache)
}

// In main loop (interactive.rs)
let (max_scroll, layout_cache) = render(frame, app);
app.update_max_scroll(max_scroll);
app.update_layout_cache(layout_cache);  // NEW
```

**Focus-Aware Rendering:**
```rust
fn render_messages(frame: &mut Frame, area: Rect, app: &App) {
    let border_style = if app.focus_messages.is_focused() {
        Style::default().fg(RUST_ORANGE)  // Focused: bright
    } else {
        Style::default().fg(Color::DarkGray)  // Unfocused: dimmed
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title("Messages");

    // ... rest of rendering
}
```

---

## 🔧 Implementation Phases

### Phase 1: Foundation - Basic Focus Management (2-3 hours)

**Goal:** Add rat-focus, implement focus for main panes (messages, input, debug)

**Tasks:**
1. Add `rat-focus = "0.1"` to `Cargo.toml` (check latest version)
2. Add focus fields to `App` struct:
   ```rust
   focus_messages: FocusFlag,
   focus_input: FocusFlag,
   focus_debug: FocusFlag,
   layout_cache: Option<LayoutCache>,
   ```
3. Create `LayoutCache` struct in app.rs
4. Implement wrapper structs with `HasFocus` trait (MessagesPaneWrapper, InputPaneWrapper, DebugPaneWrapper)
5. Update `render()` to return `(usize, LayoutCache)`
6. Update main loop (interactive.rs) to store layout cache
7. Add focus-aware border styling in render functions
8. Update event handler to build focus structure and handle events
9. Test: Tab navigation between panes, visual feedback

**Breaking Changes:**
- `render()` signature changes (return tuple)
- `App::new()` needs to initialize FocusFlags and layout_cache

**Code Example:**
```rust
// Wrapper for messages pane
pub struct MessagesPaneWrapper<'a> {
    app: &'a App,
    area: Rect,
}

impl<'a> HasFocus for MessagesPaneWrapper<'a> {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.app.focus_messages.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn navigable(&self) -> Navigation {
        Regular  // Tab-navigable
    }
}
```

### Phase 2: Overlay Z-Ordering (1-2 hours)

**Goal:** Integrate autocomplete and memory modal with z-ordering

**Tasks:**
1. Add focus fields for overlays:
   ```rust
   focus_autocomplete: FocusFlag,
   focus_memory_modal: FocusFlag,
   ```
2. Implement wrapper structs with `area_z()`:
   ```rust
   impl HasFocus for AutocompletePopupWrapper {
       fn area_z(&self) -> Option<(Rect, u16)> {
           Some((self.area, 1))  // Z-order 1
       }

       fn navigable(&self) -> Navigation {
           MouseOnly  // Not Tab-navigable
       }
   }
   ```
3. Remove manual `Clear` widget calls (rat-focus handles this)
4. Test: Click overlays to focus, Tab doesn't focus overlays

**Breaking Changes:** None (internal only)

### Phase 3: TextArea Integration (2-3 hours)

**Goal:** Properly integrate tui-textarea with rat-focus

**Solution:** Wrapper pattern - Keep `TextArea` as-is in App, use separate `focus_input` FocusFlag

```rust
pub struct InputPaneWrapper<'a> {
    app: &'a App,
    area: Rect,
}

impl HasFocus for InputPaneWrapper<'_> {
    fn focus(&self) -> FocusFlag {
        self.app.focus_input.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }
}
```

**Rendering:**
```rust
fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    // Apply focus styling to block
    let mut input_widget = app.input.clone();

    if app.focus_input.is_focused() {
        // Apply focused styling
    }

    frame.render_widget(&input_widget, area);
}
```

### Phase 4: Mouse Event Handling (1 hour)

**Goal:** Click panes to focus them

**Tasks:**
1. rat-focus handles this automatically via `area()` hit testing
2. Verify mouse clicks work with z-ordering (overlays take priority)
3. Test: Click each pane, verify focus changes

**No code changes needed** - rat-focus `HandleEvent` trait does this.

### Phase 5: Keyboard Shortcuts (30 mins)

**Goal:** Add explicit pane switching (F2/F3/F4 or Tab cycling)

**Tasks:**
1. Tab already handled by rat-focus
2. Optionally add F-key shortcuts for direct pane focus
3. Test: Tab cycles through panes in order

**Implementation:**
```rust
// In handle_key_event()
KeyCode::F(2) => {
    // Force focus to messages pane via Focus::focus_at()
}
```

---

## 🎨 Visual Design

### Focus Styling

**Focused Pane:**
- Border color: `RUST_ORANGE` (Rgb(222, 165, 132))
- Border style: Bold (optional)

**Unfocused Pane:**
- Border color: `Color::DarkGray` (Rgb(80, 80, 80))
- Border style: Normal

**Overlay (when focused):**
- Border color: `RUST_ORANGE`
- Background: Cleared (rat-focus handles via z-ordering)

**Example:**
```
┌─────────────────────────────┐  ← DarkGray (unfocused)
│ Messages                    │
│ ...                         │
└─────────────────────────────┘
┌─────────────────────────────┐  ← RUST_ORANGE (focused)
│ ✏️  Input                   │
│ █                           │
└─────────────────────────────┘
```

### Cursor Behavior

**Input pane focused:**
- Cursor visible in TextArea
- Text input active

**Input pane unfocused:**
- Cursor hidden (tui-textarea handles this)
- No text input

**Messages pane focused:**
- Scroll keys affect messages
- No cursor

---

## 🔍 Key Decisions & Trade-offs

### 1. Layout Cache Storage

**Decision:** Store `LayoutCache` in App state

**Rationale:**
- rat-focus `area()` method needs Rect values
- Layout calculated during render, needed during event handling
- Alternative (recalculate layout) is inefficient

**Trade-off:** Adds 24-32 bytes to App state, updated every frame

### 2. Pure Render Functions

**Decision:** Keep `render()` pure by returning LayoutCache

**Rationale:**
- Maintains existing architectural principle
- Layout cache is logically a return value (output of render calculations)
- Caller updates App state

**Trade-off:** Slightly more verbose main loop

### 3. TextArea Integration

**Decision:** Don't wrap TextArea, use separate FocusFlag

**Rationale:**
- TextArea is complex external widget, wrapping is invasive
- Focus state can live separately in App
- Avoids needing `&mut App` in render

**Trade-off:** Slight duplication (focus flag separate from widget state)

### 4. Overlay Navigation

**Decision:** Overlays are `MouseOnly`, not Tab-navigable

**Rationale:**
- Autocomplete already has arrow key navigation
- Memory modal already has arrow key navigation
- Tab should cycle through main panes only

**Trade-off:** Users can't Tab to overlays (but arrow keys work)

### 5. Default Focus

**Decision:** Input pane starts focused

**Rationale:**
- Most common action is typing
- Matches current implicit behavior
- Users expect to type immediately

**Trade-off:** None

---

## 🚧 Migration Strategy

### Backward Compatibility

**Breaking Changes:**
1. `render()` signature: Returns `(usize, LayoutCache)` instead of `usize`
2. `App::new()`: Requires initializing FocusFlags
3. Event handling: May need to pass layout cache

**Mitigation:**
- Phase these changes incrementally
- Keep tests passing after each phase
- Document changes in CHANGELOG.md

### Refactoring Checklist

**Files to Modify:**
- ✅ `Cargo.toml` - Add rat-focus dependency
- ✅ `app.rs` - Add focus fields, wrapper structs
- ✅ `ui.rs` - Update render signature, focus styling
- ✅ `event.rs` - Integrate FocusBuilder
- ✅ `mod.rs` - Update exports
- ✅ `interactive.rs` - Handle new render return type

**Files That Stay the Same:**
- ✅ `layout.rs` - No changes needed
- ✅ `message.rs` - No changes needed
- ✅ `keybindings.rs` - No changes needed
- ✅ `token_counter.rs` - No changes needed

### Testing Strategy

**Unit Tests:**
```rust
#[test]
fn test_focus_initialization() {
    let app = App::new(PermissionMode::default());
    assert!(app.focus_input.is_focused());  // Default focus
    assert!(!app.focus_messages.is_focused());
}

#[test]
fn test_focus_navigation() {
    let mut app = App::new(PermissionMode::default());
    // Simulate Tab key to move focus
    // Verify focus_messages becomes focused
}

#[test]
fn test_overlay_z_ordering() {
    let app = App::new(PermissionMode::default());
    app.activate_autocomplete(vec![/* items */]);

    // Verify autocomplete area_z() returns higher z than messages
}
```

**Integration Tests:**
- Click each pane, verify border color change
- Tab through panes, verify navigation order
- Click overlay, verify it captures focus
- Resize terminal, verify layout cache updates

---

## 🤔 Open Questions & Recommendations

### 1. Should scroll events respect focused pane?

**Current:** Scroll always affects messages pane
**Proposed:** Scroll affects focused pane

**Recommendation:** **Keep current behavior** (scroll always messages)

**Rationale:**
- Scrolling messages is primary use case
- Less surprising behavior (scrolling "just works")
- Input pane has its own scroll (when > 5 lines)

### 2. Should typing auto-focus input pane?

**Current:** Typing always goes to input (implicit)
**Proposed:** Typing only works when input focused

**Recommendation:** **Auto-focus input on typing**

**Implementation:**
```rust
// In handle_key_event()
if bindings.is_printable_char(&key) {
    // Auto-focus input pane
    app.focus_input.set();  // Via Focus::focus_at()
    app.insert_char(c);
}
```

**Rationale:**
- Matches user expectation
- Typing is primary interaction
- Avoids confusion ("why isn't it working?")

### 3. Do we need the layout cache at all?

**Alternative:** Recalculate layout in event handler

**Recommendation:** **Keep layout cache**

**Rationale:**
- Layout calculation is non-trivial (dynamic input height, debug panel)
- Recalculating wastes CPU cycles
- Cache is only 24-32 bytes
- rat-focus `area()` calls happen frequently (every mouse move potentially)

### 4. Should overlays steal focus or coexist?

**Current Behavior:** Autocomplete/modal overlay everything

**Proposed:** Overlay gets focus, underlying panes unfocused

**Recommendation:** **Overlays steal focus**

**Implementation:**
```rust
impl HasFocus for AutocompletePopupWrapper {
    fn navigable(&self) -> Navigation {
        MouseOnly  // Can't Tab to it, but clicking focuses it
    }
}
```

**Rationale:**
- Overlays are temporary UI state
- User is interacting with overlay, not underlying panes
- Escape key dismisses overlay, returns focus to last pane

---

## 📚 References

**Documentation:**
- rat-focus crate: https://docs.rs/rat-focus
- ratatui mouse events: https://ratatui.rs/concepts/event-handling/
- tui-textarea integration: https://docs.rs/tui-textarea

**Examples:**
- rat-focus examples: https://github.com/thscharler/rat-focus/tree/main/examples
- ratatui-widgets showcase: https://github.com/ratatui-org/ratatui/tree/main/examples

---

## 🎯 Success Criteria

**Phase 1 Complete:**
- ✅ Tab cycles through panes (messages → input → debug)
- ✅ Focused pane has bright border, unfocused dimmed
- ✅ No regression in existing functionality
- ✅ Tests pass

**Phase 2 Complete:**
- ✅ Overlays render on top (z-ordering)
- ✅ Clicking overlay focuses it
- ✅ Escape dismisses overlay
- ✅ Manual `Clear` widget removed

**Phase 3 Complete:**
- ✅ TextArea integrates with focus system
- ✅ Cursor visibility tied to focus state
- ✅ Typing auto-focuses input pane

**Full Integration:**
- ✅ All components use rat-focus
- ✅ No custom hit-testing code
- ✅ Clean separation: state (App) / focus (rat-focus) / render (ui.rs)

---

## 🚀 Final Recommendation

**Adopt rat-focus fully** instead of implementing custom focus system from MOUSE_INTERACTION_REQUIREMENTS.md.

**Benefits:**
- ✅ Battle-tested library (part of rat-salsa ecosystem)
- ✅ Z-ordering built-in (no manual `Clear` widgets)
- ✅ Mouse event routing automatic
- ✅ Tab navigation free
- ✅ Idiomatic Ratatui patterns
- ✅ Less code to maintain

**Estimated Effort:**
- Phase 1 (foundation): 2-3 hours
- Phase 2 (overlays): 1-2 hours
- Phase 3 (TextArea): 2-3 hours
- Phase 4 (mouse): 1 hour
- Phase 5 (keyboard): 30 mins

**Total: 6-9 hours** for complete focus management system vs ~10-15 hours for custom implementation.

**Architecture aligns with RustyClawd's philosophy:**
- Ruthless simplicity (use library vs build custom)
- Modular design (focus state separate from render)
- Zero-BS implementation (rat-focus is production-ready)
