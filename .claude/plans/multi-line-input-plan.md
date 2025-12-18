# Multi-Line Input Implementation Plan

## Overview

Enhance the input box to support multi-line text editing with line wrapping, scrolling, and advanced navigation.

## Discovery

**Current State:**
- Single-line `String` input with byte-offset cursor (`app.rs:92-95`)
- Simple character-level navigation (left/right)
- Basic keybindings in `keybindings.rs`
- Rendering via simple `Paragraph` widget (`ui.rs:446-476`)
- No word navigation, no multi-line support

**Key Finding:**
`tui-textarea = "0.6"` is already in `Cargo.toml` - a mature multi-line text editor widget designed for ratatui.

## Solution: Leverage tui-textarea

Instead of implementing a complex multi-line editor from scratch, use `tui-textarea::TextArea` which provides:
- ✅ Multi-line editing out of the box
- ✅ Automatic line wrapping
- ✅ Cursor management in 2D space
- ✅ Word-based navigation (Ctrl+Arrow)
- ✅ Undo/redo support
- ✅ Scrolling viewports

## Implementation Steps

### Phase 1: Replace Input State (app.rs)

**File:** `crates/cli/src/tui/app.rs`

**Changes:**
1. Add import: `use tui_textarea::TextArea;`
2. Replace fields in `App` struct:
   ```rust
   // OLD:
   input: String,
   cursor_pos: usize,

   // NEW:
   pub input: TextArea,  // ✅ Public field for rendering access
                         // ✅ NO lifetime parameter - TextArea is owned type
   ```

3. Update initialization in `App::new()`:
   ```rust
   // Configure TextArea with styling ONCE during initialization
   let mut input = TextArea::default();
   input.set_block(
       Block::default()
           .borders(Borders::ALL)
           .border_style(Style::default().fg(RUST_ORANGE))
           .title(vec![
               Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
               Span::styled(
                   "Input",
                   Style::default()
                       .fg(RUST_ORANGE)
                       .add_modifier(Modifier::BOLD),
               ),
           ])
   );
   // Note: Styling is set once here, not during rendering
   ```

4. Update accessor methods:
   ```rust
   // Return String (owned) since we're joining lines
   pub fn input(&self) -> String {
       self.input.lines().join("\n")
   }

   // Return line count for height calculation
   pub fn input_line_count(&self) -> usize {
       self.input.lines().len().max(1)  // Minimum 1 line
   }

   // Check if multi-line input (for event priority)
   pub fn has_multi_line_input(&self) -> bool {
       self.input.lines().len() > 1
   }

   // For cursor position (if needed elsewhere)
   pub fn cursor_pos(&self) -> (usize, usize) {
       self.input.cursor()  // Returns (row, col)
   }
   ```

5. Replace editing methods:
   ```rust
   // Keep existing method signatures for compatibility, delegate to TextArea

   pub fn insert_char(&mut self, c: char) {
       self.input.insert_char(c);
       self.mark_dirty();
   }

   pub fn delete_char(&mut self) {
       self.input.delete_char();
       self.mark_dirty();
   }

   pub fn backspace(&mut self) {
       self.input.delete_char();  // TextArea has only delete_char
       self.mark_dirty();
   }

   pub fn move_cursor_left(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Back);
       self.mark_dirty();
   }

   pub fn move_cursor_right(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Forward);
       self.mark_dirty();
   }

   pub fn move_cursor_to_start(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Head);
       self.mark_dirty();
   }

   pub fn move_cursor_to_end(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::End);
       self.mark_dirty();
   }

   // NEW: Multi-line navigation
   pub fn insert_newline(&mut self) {
       self.input.insert_newline();
       self.mark_dirty();
   }

   pub fn move_cursor_up(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Up);
       self.mark_dirty();
   }

   pub fn move_cursor_down(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Down);
       self.mark_dirty();
   }

   // NEW: Word navigation
   pub fn move_cursor_word_left(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::WordBack);
       self.mark_dirty();
   }

   pub fn move_cursor_word_right(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::WordForward);
       self.mark_dirty();
   }

   // NEW: Absolute navigation
   pub fn move_cursor_absolute_start(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Top);
       self.mark_dirty();
   }

   pub fn move_cursor_absolute_end(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Bottom);
       self.mark_dirty();
   }

   // NEW: Page navigation (jump to top/bottom of input)
   pub fn move_cursor_to_input_top(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Top);
       self.mark_dirty();
   }

   pub fn move_cursor_to_input_bottom(&mut self) {
       self.input.move_cursor(tui_textarea::CursorMove::Bottom);
       self.mark_dirty();
   }

   // NEW: Viewport scrolling (Ctrl+Up/Down)
   // Note: TextArea may handle this automatically - verify during implementation
   pub fn scroll_input_viewport_up(&mut self) {
       // TODO: Investigate if TextArea has scroll() method or if auto-handled
       // For now, just mark dirty in case TextArea needs repaint
       self.mark_dirty();
   }

   pub fn scroll_input_viewport_down(&mut self) {
       // TODO: Same as above
       self.mark_dirty();
   }
   ```

6. Update `submit_input()`:
   ```rust
   pub fn submit_input(&mut self) -> Option<String> {
       let text = self.input.lines().join("\n");
       if text.trim().is_empty() {
           return None;
       }

       // Reset TextArea - recreate with same styling
       let mut new_input = TextArea::default();
       new_input.set_block(
           Block::default()
               .borders(Borders::ALL)
               .border_style(Style::default().fg(RUST_ORANGE))
               .title(vec![
                   Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
                   Span::styled("Input", Style::default().fg(RUST_ORANGE).add_modifier(Modifier::BOLD)),
               ])
       );
       self.input = new_input;

       self.mark_dirty();
       Some(text)
   }
   ```

### Phase 2: Add New Keybindings (keybindings.rs)

**File:** `crates/cli/src/tui/keybindings.rs`

**Changes:**
1. Add new `KeyAction` variants:
   ```rust
   pub enum KeyAction {
       // ... existing ...

       // NEW: Multi-line input navigation
       // Note: Plain Up/Down keys keep existing ScrollUp/ScrollDown actions.
       // Context-aware handling in event.rs will check if multi-line input is active.
       CursorWordLeft,
       CursorWordRight,
       CursorAbsoluteStart,  // Ctrl+Home
       CursorAbsoluteEnd,    // Ctrl+End
       InputPageUp,          // Jump to input top
       InputPageDown,        // Jump to input bottom
       InputScrollUp,        // Ctrl+Up (viewport scroll)
       InputScrollDown,      // Ctrl+Down (viewport scroll)
       InsertNewline,        // Shift+Enter or backslash-escaped Enter
   }
   ```

2. Add new keybindings in `KeyBindings::default()`:
   ```rust
   // NOTE: Plain Up/Down keys already bound to ScrollUp/ScrollDown.
   // Event handlers will add context-awareness using app.has_multi_line_input() check.

   // Word navigation
   KeyBinding {
       key: KeyPattern::ctrl(KeyCodePattern::Left),
       action: KeyAction::CursorWordLeft,
       description: "Move cursor left by word".to_string(),
   },
   KeyBinding {
       key: KeyPattern::ctrl(KeyCodePattern::Right),
       action: KeyAction::CursorWordRight,
       description: "Move cursor right by word".to_string(),
   },

   // Absolute navigation
   KeyBinding {
       key: KeyPattern::ctrl(KeyCodePattern::Home),
       action: KeyAction::CursorAbsoluteStart,
       description: "Jump to start of all input".to_string(),
   },
   KeyBinding {
       key: KeyPattern::ctrl(KeyCodePattern::End),
       action: KeyAction::CursorAbsoluteEnd,
       description: "Jump to end of all input".to_string(),
   },

   // Page navigation
   KeyBinding {
       key: KeyPattern::plain(KeyCodePattern::PageUp),
       action: KeyAction::InputPageUp,
       description: "Jump to top of input".to_string(),
   },
   KeyBinding {
       key: KeyPattern::plain(KeyCodePattern::PageDown),
       action: KeyAction::InputPageDown,
       description: "Jump to bottom of input".to_string(),
   },

   // Viewport scrolling
   KeyBinding {
       key: KeyPattern::ctrl(KeyCodePattern::Up),
       action: KeyAction::InputScrollUp,
       description: "Scroll input viewport up".to_string(),
   },
   KeyBinding {
       key: KeyPattern::ctrl(KeyCodePattern::Down),
       action: KeyAction::InputScrollDown,
       description: "Scroll input viewport down".to_string(),
   },

   // Newline insertion
   KeyBinding {
       key: KeyPattern {
           code: KeyCodePattern::Enter,
           modifiers: KeyModifiers::SHIFT,
       },
       action: KeyAction::InsertNewline,
       description: "Insert newline (Shift+Enter)".to_string(),
   },
   ```

3. Add helper for Shift modifier:
   ```rust
   impl KeyPattern {
       pub fn shift(code: KeyCodePattern) -> Self {
           Self {
               code,
               modifiers: KeyModifiers::SHIFT,
           }
       }
   }
   ```

### Phase 3: Update Event Handling (event.rs)

**File:** `crates/cli/src/tui/event.rs`

**Changes:**
1. Update existing `ScrollUp`/`ScrollDown` handlers to add context-aware multi-line input support:
   ```rust
   match action {
       // ... existing cases ...

       // UPDATED: Add context-aware handling for multi-line input
       KeyAction::ScrollUp(n) => {
           if !app.is_streaming() {
               // Priority: memory modal > autocomplete > multi-line input > message scrolling
               if app.memory_modal_active() {
                   app.memory_modal_prev();
               } else if app.autocomplete_active() {
                   app.autocomplete_prev();
               } else if app.has_multi_line_input() {
                   // NEW: Move cursor up in multi-line input
                   app.move_cursor_up();
               } else {
                   // Existing: Scroll message history
                   app.scroll_up(*n);
               }
           }
       }

       // UPDATED: Add context-aware handling for multi-line input
       KeyAction::ScrollDown(n) => {
           if !app.is_streaming() {
               // Priority: memory modal > autocomplete > multi-line input > message scrolling
               if app.memory_modal_active() {
                   app.memory_modal_next();
               } else if app.autocomplete_active() {
                   app.autocomplete_next();
               } else if app.has_multi_line_input() {
                   // NEW: Move cursor down in multi-line input
                   app.move_cursor_down();
               } else {
                   // Existing: Scroll message history
                   app.scroll_down(*n);
               }
           }
       }

       // NEW: Word navigation
       KeyAction::CursorWordLeft => {
           if !app.is_streaming() {
               app.move_cursor_word_left();
           }
       }

       KeyAction::CursorWordRight => {
           if !app.is_streaming() {
               app.move_cursor_word_right();
           }
       }

       // NEW: Absolute navigation
       KeyAction::CursorAbsoluteStart => {
           if !app.is_streaming() {
               app.move_cursor_absolute_start();
           }
       }

       KeyAction::CursorAbsoluteEnd => {
           if !app.is_streaming() {
               app.move_cursor_absolute_end();
           }
       }

       // NEW: Page navigation
       KeyAction::InputPageUp => {
           if !app.is_streaming() {
               app.move_cursor_to_input_top();
           }
       }

       KeyAction::InputPageDown => {
           if !app.is_streaming() {
               app.move_cursor_to_input_bottom();
           }
       }

       // NEW: Viewport scrolling (Ctrl+Up/Down)
       KeyAction::InputScrollUp => {
           if !app.is_streaming() {
               app.scroll_input_viewport_up();
           }
       }

       KeyAction::InputScrollDown => {
           if !app.is_streaming() {
               app.scroll_input_viewport_down();
           }
       }

       // NEW: Newline insertion
       KeyAction::InsertNewline => {
           if !app.is_streaming() {
               app.insert_newline();
           }
       }
   }
   ```

2. **Special handling for backslash-escaped Enter:**
   Update `handle_key_event()` to check for `\<Enter>` sequence:
   ```rust
   fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<EventResult> {
       // ... existing code ...

       // Check if last character is backslash and Enter is pressed
       if matches!(key.code, KeyCode::Enter) && !key.modifiers.contains(KeyModifiers::SHIFT) {
           let input = app.input();
           if input.ends_with('\\') {
               // Remove trailing backslash and insert newline
               app.backspace();
               app.insert_newline();
               return Ok(EventResult::Continue);
           }
       }

       // ... rest of existing code ...
   }
   ```

### Phase 4: Update Rendering (ui.rs)

**File:** `crates/cli/src/tui/ui.rs`

**Changes:**
1. Update `split_main()` signature to accept `&App` for dynamic height calculation:
   ```rust
   fn split_main(main: Rect, app: &App) -> (Rect, Rect) {
       // Calculate input height dynamically (1-5 lines)
       let input_height = calculate_input_height(app);

       Layout::default()
           .direction(Direction::Vertical)
           .constraints([
               Constraint::Min(0),           // Messages area
               Constraint::Length(input_height),  // Input area (dynamic)
           ])
           .split(main)
           .into()
   }

   fn calculate_input_height(app: &App) -> u16 {
       let line_count = app.input_line_count();
       // Min 3 (1 line + 2 borders), Max 7 (5 lines + 2 borders)
       (line_count + 2).clamp(3, 7)
   }
   ```

2. Update `render_input()` to use TextArea widget:
   ```rust
   fn render_input(frame: &mut Frame, area: Rect, app: &App) {
       // CORRECT: Render TextArea directly with immutable borrow
       // TextArea implements Widget trait
       // Styling was configured once during initialization (App::new)
       frame.render_widget(&app.input, area);

       // Cursor is handled automatically by TextArea
       // No manual cursor positioning needed
   }
   ```

3. Update `render()` function to pass app to `split_main()`:
   ```rust
   pub fn render(frame: &mut Frame, app: &App) {
       let main = frame.area();
       let (messages_area, input_area) = split_main(main, app);  // Pass app

       render_messages(frame, messages_area, app);
       render_input(frame, input_area, app);
       // ... rest of rendering ...
   }
   ```

4. Autocomplete/memory modal positioning:
   - These need to remain above the input area
   - No changes needed if they use `input_area` as reference
   - Dynamic input height is transparent to modals

### Phase 5: Handle Edge Cases

**File:** `crates/cli/src/tui/compat.rs`

**Changes:**
1. Update autocomplete trigger logic:
   ```rust
   fn update_autocomplete_if_needed(&mut self) {
       let input = self.app.input();

       // Only trigger on lines starting with '/'
       let current_line = get_current_line(input, self.app.cursor_pos());
       if !current_line.starts_with('/') {
           self.app.clear_autocomplete();
           return;
       }

       // ... rest of autocomplete logic ...
   }
   ```

2. Update memory modal trigger:
   ```rust
   fn update_memory_modal_if_needed(&mut self) {
       let input = self.app.input();

       // Only trigger on lines starting with '#'
       let current_line = get_current_line(input, self.app.cursor_pos());
       if !current_line.starts_with('#') {
           self.app.clear_memory_modal();
           return;
       }

       // ... rest of memory modal logic ...
   }
   ```

## Implementation Priority

1. **Phase 1 (Core):** Replace input state with TextArea
2. **Phase 2 (Keybindings):** Add new keybindings
3. **Phase 3 (Events):** Wire up event handling
4. **Phase 4 (Rendering):** Update rendering with dynamic height
5. **Phase 5 (Polish):** Handle edge cases with autocomplete/memory

## Testing Strategy

1. **Unit Tests:**
   - Word boundary detection
   - Line counting
   - Viewport offset calculation

2. **Integration Tests:**
   - Multi-line input with wrapping
   - Shift+Enter newline insertion
   - Backslash-escape newline
   - Ctrl+Arrow word navigation
   - Page Up/Down navigation
   - Ctrl+Home/End absolute navigation
   - Scrolling with > 5 lines

3. **Manual Testing:**
   - Type long text, verify wrapping
   - Test all keybindings
   - Verify autocomplete still works
   - Verify memory modal still works
   - Test with CJK/emoji characters

## Risks & Mitigations

**Risk 1:** TextArea API might not match current interface
- **Mitigation:** Add adapter methods in App to maintain compatibility

**Risk 2:** TextArea cursor might not work with autocomplete/memory modal
- **Mitigation:** Extract current line/position helpers

**Risk 3:** Dynamic height might cause layout issues
- **Mitigation:** Clamp height to [1, 5] lines strictly

**Risk 4:** Backslash-escape might conflict with other uses
- **Mitigation:** Only trigger on trailing backslash immediately before Enter

## Success Criteria

- ✅ Multi-line input with visual line wrapping
- ✅ Input box expands to max 5 lines
- ✅ Scrolling when > 5 lines
- ✅ Shift+Enter inserts newline
- ✅ `\<Enter>` inserts newline
- ✅ Ctrl+Arrow moves by words
- ✅ Arrow Up/Down navigates lines
- ✅ Page Up/Down jumps to input top/bottom
- ✅ Ctrl+Home/End jumps to absolute start/end
- ✅ Ctrl+Up/Down scrolls viewport (when > 5 lines)
- ✅ Autocomplete still works on lines starting with `/`
- ✅ Memory modal still works on lines starting with `#`
- ✅ All existing tests pass

## Alternative Considered

**Custom Implementation:** Build multi-line editor from scratch
- **Pros:** Full control, no dependencies
- **Cons:**
  - Complex (2000+ lines of code)
  - Bug-prone (cursor positioning, word boundaries, etc.)
  - Maintenance burden
  - Reinventing wheel
- **Decision:** Rejected - tui-textarea is battle-tested and already available

## Philosophy Alignment

✅ **Ruthless Simplicity:** Leverage existing library instead of custom implementation
✅ **Modular Design:** TextArea is self-contained, regeneratable component
✅ **Zero-BS:** TextArea is production-ready, not a stub
✅ **Library over Custom:** Perfect use case - complex problem, mature solution
