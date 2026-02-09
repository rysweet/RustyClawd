# Extended Thinking Phase Support - TUI Implementation

## Overview

This document describes the Extended Thinking Phase Support feature implemented for RustyClawd's Terminal User Interface (TUI).

## What is Extended Thinking?

Extended Thinking is Claude's capability to show its internal reasoning process before providing a final answer. When enabled, Claude sends `ContentBlock::Thinking` blocks containing its chain-of-thought reasoning.

## TUI Implementation

### Visual Feedback

When Claude enters extended thinking mode, the status bar displays:

```
⣾⣀⣀⣀⣀⣀⣀⣀ Extended thinking...
⣿⣦⣀⣀⣀⣀⣀⣀ Extended thinking (5s)...
```

**Features:**
- Animated shimmer effect (8-frame flowing wave pattern)
- Duration display (updates in real-time)
- Magenta color to distinguish from regular streaming (yellow)
- Synchronized animation across all UI components

### Input Blocking

During extended thinking:
- All keyboard input is blocked (typing, navigation, etc.)
- **Exception**: Ctrl+C always works for interruption
- **Exception**: Ctrl+D always works for exit
- User sees debug message: "⚠️ Input blocked during extended thinking (Ctrl+C to interrupt)"

This prevents users from accidentally interrupting Claude's reasoning process while still allowing intentional interruption.

## Architecture

### Three-Layer Brick Design

Following the project's brick philosophy, the implementation consists of three self-contained modules:

#### 1. Core State Management (`thinking_state.rs`)

**Purpose**: Thread-safe state tracking for thinking phases

**Public API:**
- `ThinkingState` - Main state struct
- `ThinkingPhase` - Enum (Idle, Thinking, Responding)
- Methods: `start_thinking()`, `append_thinking()`, `stop_thinking()`

**Implementation:**
- Uses `Arc<RwLock<>>` for thread safety
- Tracks duration with `Instant`
- Accumulates thinking content

#### 2. Visual Indicator (`thinking_indicator.rs`)

**Purpose**: Shimmer animation and status text generation

**Public API:**
- `render_thinking_indicator(duration)` - Returns status text with animation

**Implementation:**
- 8-frame shimmer animation using Braille patterns
- Synchronized via system clock (100ms frame time)
- Human-readable duration formatting (e.g., "1m 23s")

#### 3. Input Guard (`input_guard.rs`)

**Purpose**: Input filtering during thinking phases

**Public API:**
- `should_block_input(is_thinking, key_event)` - Returns true if input should be blocked
- `get_blocked_input_message()` - Returns user-facing message

**Implementation:**
- Blocks all input when thinking
- Always allows Ctrl+C and Ctrl+D
- Clear error messaging

## Integration Points

### Stream Event Handling

**File**: `crates/cli/src/interactive.rs`

New `StreamingChannelEvent` variants:
- `ExtendedThinkingStarted` - Sent when `ContentBlockStart::Thinking` received
- `ExtendedThinkingDelta { thinking }` - Sent when `ThinkingDelta` received
- `ExtendedThinkingStopped` - Sent when `ContentBlockStop` received

Stream handler tracks thinking blocks and sends appropriate events to the main event loop.

### TUI App State

**File**: `crates/cli/src/tui/app.rs`

`StreamingState` extended with:
- `thinking_state: ThinkingState` - Extended thinking state tracker

New methods on `App`:
- `is_extended_thinking()` - Check if in extended thinking phase
- `start_extended_thinking()` - Enter extended thinking phase
- `append_thinking_content(content)` - Append thinking content
- `stop_extended_thinking()` - Exit extended thinking phase
- `thinking_duration()` - Get duration of current thinking phase

### Event Handling

**File**: `crates/cli/src/tui/event.rs`

`handle_key_event()` modified to:
1. Check if in extended thinking phase
2. Call `should_block_input()` to determine if input should be blocked
3. Block input if necessary (except Ctrl+C/Ctrl+D)
4. Show user-facing message about blocked input

### UI Rendering

**File**: `crates/cli/src/tui/ui.rs`

Status bar rendering enhanced to:
1. Check `is_extended_thinking()` state
2. Call `render_thinking_indicator()` to get animated status text
3. Use magenta color for extended thinking (vs yellow for regular streaming)
4. Display duration alongside shimmer animation

### Compatibility Layer

**File**: `crates/cli/src/tui/compat.rs`

`TuiState` wrapper extended with:
- `start_extended_thinking()` - Forward to `App`
- `append_thinking_content(content)` - Forward to `App`
- `stop_extended_thinking()` - Forward to `App`

## Testing

### Unit Tests

**12 tests across 3 modules, all passing:**

**thinking_state.rs** (2 tests):
- Lifecycle test (Idle → Thinking → Responding → Idle)
- Duration tracking test

**thinking_indicator.rs** (4 tests):
- Shimmer frame count validation
- Current frame retrieval
- Duration formatting (0s, 59s, 1m 05s, etc.)
- Indicator rendering with/without duration

**input_guard.rs** (6 tests):
- Allow input when not thinking
- Block regular keys when thinking
- Allow Ctrl+C when thinking
- Allow Ctrl+D when thinking
- Block other Ctrl keys when thinking
- Blocked input message validation

### Integration Tests

**Existing app tests still pass (no regression):**
- `test_streaming_lifecycle` - Verifies streaming flow still works
- `test_input_submission` - Verifies input handling still works
- `test_cursor_movement` - Verifies navigation still works
- `test_unicode_input` - Verifies Unicode handling still works
- `test_empty_input_not_submitted` - Verifies validation still works

## Usage

### Enabling Extended Thinking

Extended thinking is automatically enabled when using models that support it (e.g., `claude-sonnet-4-5-20250929`).

The API request includes:
```rust
CreateMessageRequest::new(...)
    .with_thinking(4000) // 4000 token budget for thinking
```

### User Experience

1. User sends a request
2. Status bar shows: `⣾⣀⣀⣀⣀⣀⣀⣀ Extended thinking...`
3. Shimmer animation plays, duration updates
4. User input is blocked (except Ctrl+C)
5. When thinking completes, input is unblocked
6. Status bar transitions to normal streaming indicator

## Performance

- **Memory**: Minimal overhead (single `Arc<RwLock<>>` per stream)
- **CPU**: Animation updates every 100ms (negligible)
- **Thread safety**: RwLock allows concurrent reads during rendering

## Future Enhancements

Potential improvements:
1. **Configurable animation speed** - Let users adjust frame rate
2. **Alternative animation styles** - Different visual effects
3. **Thinking content preview** - Show snippet of reasoning in sidebar
4. **Thinking history** - Store and review past reasoning
5. **Keyboard shortcut to toggle thinking visibility** - Show/hide reasoning blocks

## Implementation Notes

### Why Thread-Safe State?

The `ThinkingState` uses `Arc<RwLock<>>` because:
- Stream handler runs in background task (different thread)
- Main event loop needs to query state for rendering
- Multiple readers (rendering) need concurrent access
- Single writer (stream handler) needs exclusive access

### Why Shimmer Animation?

The flowing shimmer effect:
- Indicates active processing (not frozen)
- Doesn't distract from content
- Synchronized across UI prevents visual noise
- Uses Braille patterns for smooth appearance

### Why Block Input?

Blocking input during thinking:
- Prevents accidental interruption of reasoning
- Signals to user that Claude is actively thinking
- Ctrl+C exception allows intentional interruption
- Clear messaging explains why input is blocked

## See Also

- [Extended Thinking Example](/home/azureuser/src/RustyClawd/crates/core/examples/extended_thinking.rs) - API usage examples
- [Feature Parity Summary](/home/azureuser/src/RustyClawd/docs/FEATURE_PARITY_SUMMARY.md) - Extended thinking at API level
- [Tool Use Examples](/home/azureuser/src/RustyClawd/docs/reference/TOOL_USE_EXAMPLES.md) - Complete API reference

---

**Implementation Date**: 2026-02-09
**Status**: ✅ Complete and tested
**Test Coverage**: 12/12 tests passing (100%)
