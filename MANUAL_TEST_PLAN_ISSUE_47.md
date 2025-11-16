# Manual Test Plan: TUI Streaming (Issue #47)

## Build Information
- **Branch**: `feat/tui-streaming-and-tool-visibility`
- **Build Date**: 2025-11-16
- **Binary**: `target/release/rusty chat`

## Test Objectives
Verify that TUI streaming implementation works correctly and eliminates frozen screen issues.

## Prerequisites
- Release binary built successfully: ✅
- All unit tests passing: ✅ (235 tests)
- All clippy checks passing: ✅
- Code formatted correctly: ✅

## Test Cases

### Test 1: Basic Streaming (Simple Use Case)
**Objective**: Verify text appears word-by-word without frozen screen

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Count from 1 to 10"
3. Observe streaming behavior

**Expected Results**:
- [ ] Text appears incrementally (word-by-word or chunk-by-chunk)
- [ ] No frozen screen during response
- [ ] Status bar shows "Streaming..." during response
- [ ] Status bar returns to "Ready" when complete

**Pass Criteria**: Text streams smoothly without any freeze

---

### Test 2: Tool Use Visibility (Complex Use Case)
**Objective**: Verify tool execution works and is visible during streaming

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "What files are in the current directory?"
3. Observe tool execution

**Expected Results**:
- [ ] Tool execution visible (e.g., "[Tool: Bash] running...")
- [ ] Tool results displayed in TUI
- [ ] Streaming continues after tool execution
- [ ] No frozen screen during tool execution
- [ ] Status bar shows "Executing tool: Bash" during tool call

**Pass Criteria**: Tool execution visible and streaming resumes after

---

### Test 3: Multiple Tool Calls
**Objective**: Verify multiple tool calls work in sequence

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Read the README.md file and tell me how many lines it has"
3. Observe multiple tool executions

**Expected Results**:
- [ ] First tool (Read) executes and shows result
- [ ] Second tool (Bash wc) executes and shows result
- [ ] Streaming response includes both tool results
- [ ] No frozen screen during entire process

**Pass Criteria**: Multiple tools execute successfully with visible progress

---

### Test 4: Long Response Handling
**Objective**: Verify streaming works for long responses

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Write me a 500-word essay about Rust programming"
3. Observe streaming of long content

**Expected Results**:
- [ ] Text streams continuously
- [ ] Auto-scroll keeps latest text visible
- [ ] No performance degradation
- [ ] Can scroll up to read earlier content
- [ ] Scroll returns to bottom with new content

**Pass Criteria**: Long content streams smoothly without issues

---

### Test 5: Error Handling
**Objective**: Verify graceful error handling during streaming

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message that might trigger error (e.g., network issue simulation)
3. OR: Send invalid tool command
4. Observe error behavior

**Expected Results**:
- [ ] Error message appears immediately
- [ ] Partial response preserved if any
- [ ] Status bar shows error state
- [ ] Can continue conversation after error

**Pass Criteria**: Errors handled gracefully without crash

---

### Test 6: Ctrl+C During Streaming
**Objective**: Verify Ctrl+C stops streaming gracefully

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message that generates long response
3. Press Ctrl+C mid-stream
4. Observe behavior

**Expected Results**:
- [ ] Streaming stops immediately
- [ ] TUI remains responsive
- [ ] Can continue or exit cleanly
- [ ] No corrupted state

**Pass Criteria**: Ctrl+C stops stream without corruption

---

### Test 7: Conversation History Maintained
**Objective**: Verify context maintained across turns

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "My name is Alice"
3. Wait for response
4. Send message: "What's my name?"
5. Observe response

**Expected Results**:
- [ ] Claude remembers "Alice" from previous turn
- [ ] Both messages visible in history
- [ ] Context correctly maintained

**Pass Criteria**: Multi-turn conversation works correctly

---

### Test 8: Regression Check (Existing Functionality)
**Objective**: Verify existing TUI features still work

**Steps**:
1. Run `./target/release/rusty chat`
2. Test slash commands: `/help`, `/stats`, `/history`
3. Test keyboard navigation: PageUp, PageDown, Arrow keys
4. Test autocomplete: type `/h` and press Tab
5. Exit with `/exit` or Ctrl+D

**Expected Results**:
- [ ] All slash commands work
- [ ] Keyboard navigation works
- [ ] Autocomplete works
- [ ] Exit works cleanly

**Pass Criteria**: All existing functionality intact

---

## Test Results

### Tester Information
- **Name**: _____________
- **Date**: _____________
- **Environment**: _____________

### Overall Results
- Tests Passed: __ / 8
- Tests Failed: __ / 8
- Critical Issues Found: __
- Minor Issues Found: __

### Notes
(Add any observations, issues, or unexpected behavior here)

---

## Sign-Off

**Tested By**: ____________________
**Date**: ____________________
**Status**: ☐ PASS / ☐ FAIL / ☐ PASS WITH ISSUES

**Recommendation**: ☐ Ready to Merge / ☐ Needs Fixes
