# Manual Test Plan: Tool Visibility (Issue #48)

## Build Information
- **Branch**: `feat/tui-streaming-and-tool-visibility`
- **Build Date**: 2025-11-16
- **Binary**: `target/release/rusty chat`
- **Dependencies**: Issue #47 (streaming) completed

## Test Objectives
Verify that tool execution is visible in real-time with icons, parameters, and results.

## Prerequisites
- Release binary built successfully: ✅
- All unit tests passing: ✅ (1171+ tests)
- All clippy checks passing: ✅
- Code formatted correctly: ✅
- Issue #47 (streaming) working: ✅

## Test Cases

### Test 1: Basic Tool Visibility (Bash)
**Objective**: Verify Bash tool shows with icon and result

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "run ls command"
3. Observe tool display

**Expected Results**:
- [ ] 🔧 icon appears with "Using Bash: ls"
- [ ] Command shown: "Command: ls"
- [ ] Result shows: "✓ Completed (exit code: 0, X bytes output)"
- [ ] No frozen screen

**Pass Criteria**: Tool execution visible with icon and completion status

---

### Test 2: Read Tool Visibility
**Objective**: Verify Read tool shows file information

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Read the README.md file"
3. Observe tool display

**Expected Results**:
- [ ] 📄 icon appears with "Using Read: README.md"
- [ ] File path shown truncated if long
- [ ] Result shows: "✓ Read X lines"
- [ ] Actual content follows (from Claude)

**Pass Criteria**: File reading visible with line count

---

### Test 3: Write Tool Visibility
**Objective**: Verify Write tool shows file creation

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Create a test file called test.txt with 'Hello World'"
3. Observe tool display

**Expected Results**:
- [ ] ✏️ icon appears with "Using Write: test.txt"
- [ ] Byte count shown: "Writing X bytes"
- [ ] Result shows: "✓ File written successfully"

**Pass Criteria**: File writing visible with success indicator

---

### Test 4: Glob Tool Visibility
**Objective**: Verify Glob shows pattern and results

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Find all rust files in this directory"
3. Observe tool display

**Expected Results**:
- [ ] 🔍 icon appears with "Using Glob: **/*.rs"
- [ ] Pattern shown clearly
- [ ] Result shows: "✓ Found X file(s)"

**Pass Criteria**: Glob search visible with file count

---

### Test 5: Multiple Sequential Tools
**Objective**: Verify multiple tools display in order

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Read the README.md and tell me how many lines it has"
3. Observe multiple tool executions

**Expected Results**:
- [ ] First tool (Read) shows with 📄 icon
- [ ] First tool completes with line count
- [ ] Streaming response follows
- [ ] All tools remain visible in history

**Pass Criteria**: Sequential tools visible without overlap

---

### Test 6: Tool Error Visibility
**Objective**: Verify errors display clearly

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Read a file that doesn't exist: /nonexistent.txt"
3. Observe error display

**Expected Results**:
- [ ] 📄 icon appears for Read tool
- [ ] Error shown with ✗ symbol
- [ ] Error message truncated if long (80 chars)
- [ ] Claude can respond to the error

**Pass Criteria**: Errors visible and don't break conversation

---

### Test 7: Agent (Task) Tool Visibility
**Objective**: Verify Task tool shows agent type

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message that triggers agent use (if applicable)
3. Observe agent tool display

**Expected Results**:
- [ ] 🤖 icon appears with "Using Task"
- [ ] Agent type shown (e.g., "Running architect agent")
- [ ] Description shown
- [ ] Completion indicated

**Pass Criteria**: Agent execution visible with type

---

### Test 8: Grep Tool Visibility
**Objective**: Verify Grep shows pattern and match count

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Search for the word 'TODO' in all rust files"
3. Observe Grep display

**Expected Results**:
- [ ] 🔎 icon appears with "Using Grep"
- [ ] Pattern shown: "TODO"
- [ ] Result shows: "✓ Found X match(es)"

**Pass Criteria**: Search pattern and results visible

---

### Test 9: Edit Tool Visibility
**Objective**: Verify Edit shows file and changes

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message: "Edit test.txt to replace 'Hello' with 'Goodbye'"
3. Observe Edit display

**Expected Results**:
- [ ] ✂️ icon appears with "Using Edit: test.txt"
- [ ] Change description shown
- [ ] Result shows: "✓ File edited successfully"

**Pass Criteria**: Edit operation visible with target file

---

### Test 10: Tool Visibility with Streaming
**Objective**: Verify tool display doesn't break streaming

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message requiring tool + long response
3. Observe both tool and streaming

**Expected Results**:
- [ ] Tool executes and shows before response
- [ ] Streaming text follows tool completion
- [ ] No overlap or garbled display
- [ ] Both remain readable in history

**Pass Criteria**: Tools and streaming work together seamlessly

---

### Test 11: Truncation Behavior
**Objective**: Verify long strings truncate properly

**Steps**:
1. Run `./target/release/rusty chat`
2. Send message with very long command or path
3. Observe truncation

**Expected Results**:
- [ ] Long strings show "..." ellipsis
- [ ] No line wrapping issues
- [ ] Display remains clean
- [ ] Information still useful

**Pass Criteria**: Long content truncates gracefully

---

### Test 12: Icon Display Compatibility
**Objective**: Verify emoji icons display correctly

**Steps**:
1. Check terminal supports UTF-8 and emojis
2. Run through various tool types
3. Verify all icons visible

**Expected Results**:
- [ ] All 13 tool icons display correctly
- [ ] Icons aligned properly
- [ ] No garbled characters
- [ ] Fallback icon (🔹) works

**Pass Criteria**: All icons render correctly

---

## Test Results

### Tester Information
- **Name**: _____________
- **Date**: _____________
- **Terminal**: _____________
- **Environment**: _____________

### Overall Results
- Tests Passed: __ / 12
- Tests Failed: __ / 12
- Critical Issues Found: __
- Minor Issues Found: __

### Notes
(Add any observations, issues, or unexpected behavior here)

---

## Comparison: Before vs After

### Before (Issue #47 only):
```
System> [Tool: Bash]
System> [Tool Result: Bash]
{
  "success": true,
  "stdout": "file1.txt\nfile2.txt\n",
  "exit_code": 0
}
```

### After (Issue #47 + #48):
```
System> 🔧 Using Bash: ls
System>   Command: ls
System>   ✓ Completed (exit code: 0, 245 bytes output)
```

---

## Sign-Off

**Tested By**: ____________________
**Date**: ____________________
**Status**: ☐ PASS / ☐ FAIL / ☐ PASS WITH ISSUES

**Recommendation**: ☐ Ready to Merge / ☐ Needs Fixes

---

## Integration Notes

**Combined Features (Issues #47 + #48)**:
- Streaming text responses (word-by-word)
- Tool execution visibility (icons and status)
- Real-time progress updates
- No frozen screens
- Professional TUI experience

**Success Criteria for Both Issues**:
- Streaming works: Text appears incrementally
- Tools visible: Icons, params, results shown
- No regressions: Previous features still work
- High quality: Professional, polished UX
