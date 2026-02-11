# RustyClawd Comprehensive Testing Plan

## Session Date: 2026-02-11

## Executive Summary

This document outlines a comprehensive testing strategy for RustyClawd, focusing on:
1. **Testing recent features** (last 2 weeks of development)
2. **Edge cases and error conditions**
3. **Performance and stress testing**
4. **Integration testing across features**
5. **Regression testing for known issues**

## Recent Features Requiring Testing (Last 2 Weeks)

### 1. **Extended Thinking Phase Support (#336)**
- Feature: TUI displays extended thinking in real-time
- Test Scenarios Needed:
  - [ ] Extended thinking displays correctly in TUI
  - [ ] Thinking tokens are counted properly
  - [ ] UI updates smoothly during long thinking
  - [ ] Cancel works during thinking phase
  - [ ] Thinking phase with/without output
  - [ ] Multiple thinking phases in one response

### 2. **Task Management System (#335)**
- Feature: TodoWrite tool with dependency tracking
- Test Scenarios Needed:
  - [ ] Create tasks with dependencies
  - [ ] Update task status (pending → in_progress → completed)
  - [ ] Dependency validation (can't complete if deps not done)
  - [ ] Task list display in TUI
  - [ ] Concurrent task operations
  - [ ] Invalid task status transitions
  - [ ] Circular dependency detection

### 3. **Session Resume Enhancements (#334)**
- Feature: PR linking, lazy loading, UX improvements
- Test Scenarios Needed:
  - [ ] Resume session from history
  - [ ] PR links appear in session resume UI
  - [ ] Lazy loading works for long sessions
  - [ ] Search through old sessions
  - [ ] Resume corrupted session
  - [ ] Resume with missing git info
  - [ ] Resume from different branch

### 4. **Agent Teams Multi-Agent Collaboration (#333)**
- Feature: Multiple agents working together
- Test Scenarios Needed:
  - [ ] Spawn multiple agents
  - [ ] Agent-to-agent communication
  - [ ] Shared context between agents
  - [ ] Agent failure doesn't crash others
  - [ ] Agent coordination on complex task
  - [ ] Kill specific agent in team
  - [ ] Resource limits per agent

### 5. **Memory System with SQLite Backend (#332)**
- Feature: Automatic memory persistence
- Test Scenarios Needed:
  - [ ] Memory saves automatically
  - [ ] Memory loads on startup
  - [ ] Search memories
  - [ ] Memory size limits
  - [ ] Corrupted database recovery
  - [ ] Memory exports/imports
  - [ ] Memory across sessions
  - [ ] Memory pruning/cleanup

### 6. **Fast Mode Support for Opus 4.6 (#331)**
- Feature: --fast flag for faster responses
- Test Scenarios Needed:
  - [ ] Fast mode enables correctly
  - [ ] Performance difference measurable
  - [ ] Fast mode with complex queries
  - [ ] Toggle between normal/fast
  - [ ] Fast mode with tools
  - [ ] Fast mode error handling

### 7. **Slash Command Argument Preservation (#338)**
- Feature: Fix for ultrathink prompt truncation
- Test Scenarios Needed:
  - [ ] Commands without placeholders preserve args
  - [ ] Commands with placeholders still work
  - [ ] Long arguments don't truncate
  - [ ] Special characters in arguments
  - [ ] Multiple arguments

### 8. **Branding Removal (#337)**
- Feature: Remove Claude branding from UI
- Test Scenarios Needed:
  - [ ] No "Claude" text in UI
  - [ ] No Claude-specific terminology
  - [ ] Help text uses "Rusty" branding
  - [ ] Error messages use correct branding

## Existing Features Requiring More Testing

### Tool System
- [ ] All tools work individually
- [ ] Tool chaining (one tool → another)
- [ ] Tool with missing permissions
- [ ] Tool with invalid arguments
- [ ] Tool timeout handling
- [ ] Tool cancellation mid-execution
- [ ] Concurrent tool execution

### Permission System
- [ ] Ask mode blocks tools correctly
- [ ] AutoAccept mode allows tools
- [ ] Plan mode shows plan without execution
- [ ] Shift+Tab cycles modes correctly
- [ ] Permission modal displays correctly
- [ ] Wildcard permissions (mcp__server__*)
- [ ] Permission persistence across sessions

### Slash Commands
- [ ] /help displays all commands
- [ ] /analyze works on files/dirs
- [ ] /commit-push-pr workflow
- [ ] /permissions shows permission UI
- [ ] /exit exits cleanly
- [ ] Custom slash commands
- [ ] Invalid slash commands error gracefully

### MCP Integration
- [ ] MCP servers start correctly
- [ ] MCP tools load and execute
- [ ] MCP server crash recovery
- [ ] list_changed notifications
- [ ] Schema validation filters bad tools
- [ ] structuredContent field support

### Skills System
- [ ] Skills load from directory
- [ ] Skills execute with context
- [ ] ${CLAUDE_PLUGIN_ROOT} variable substitution
- [ ] Skills with dependencies
- [ ] Skills with errors

### Background Agents
- [ ] run_in_background spawns agent
- [ ] AgentOutput tool retrieves results
- [ ] Multiple background agents
- [ ] Background agent timeout
- [ ] Kill background agent

## Edge Cases and Error Conditions

### Input Handling
- [ ] Empty input
- [ ] Extremely long input (>100KB)
- [ ] Binary data in input
- [ ] Unicode/emoji in input
- [ ] Rapid input (typing fast)
- [ ] Paste large text
- [ ] Special control characters

### Resource Limits
- [ ] Very long conversations (>1000 turns)
- [ ] Large file operations (>1GB)
- [ ] Many concurrent tools
- [ ] Memory exhaustion scenarios
- [ ] Disk space exhaustion
- [ ] CPU saturation

### Network Issues
- [ ] API timeout
- [ ] API rate limiting
- [ ] API errors (4xx, 5xx)
- [ ] Network disconnection
- [ ] Retry logic
- [ ] Partial response handling

### File System Issues
- [ ] Read-only file system
- [ ] Missing permissions
- [ ] Deleted files mid-operation
- [ ] Symlink handling
- [ ] Special files (devices, pipes)
- [ ] File locking conflicts

### Terminal/TUI Issues
- [ ] Terminal resize during operation
- [ ] Very small terminal (20x10)
- [ ] Very large terminal (300x100)
- [ ] Color support disabled
- [ ] UTF-8 not supported
- [ ] Terminal disconnect

## Performance Testing

### Response Time
- [ ] Baseline response time for simple queries
- [ ] Response time with tools
- [ ] Response time with extended thinking
- [ ] Response time degradation over long session

### Memory Usage
- [ ] Memory usage baseline
- [ ] Memory growth over long session
- [ ] Memory after tool execution
- [ ] Memory leaks detection

### Startup Time
- [ ] Cold start time
- [ ] Startup with MCP servers
- [ ] Startup with many skills
- [ ] Startup with corrupted config

## Integration Testing

### Feature Combinations
- [ ] Memory + Agent Teams
- [ ] Task Management + Session Resume
- [ ] Fast Mode + Extended Thinking
- [ ] Background Agents + Tool System
- [ ] Permission System + MCP Tools
- [ ] Slash Commands + Skills

### Workflow Testing
- [ ] Complete PR creation workflow
- [ ] Complete code review workflow
- [ ] Complete debugging workflow
- [ ] Complete refactoring workflow

## Regression Testing

### Known Issues to Verify Fixed
- [ ] PATH inheritance (recent fix)
- [ ] Session auto-detection
- [ ] execute_command action handling
- [ ] YAML parsing errors
- [ ] Color parsing issues
- [ ] Prompt truncation (#338)

## Test Infrastructure Improvements Needed

### Current Issues with Test Framework
1. **Binary name mismatch**: Tests expect `claude` but binary is `rusty`
2. **Test scenarios outdated**: Many scenarios need updating
3. **No automated test runs**: Tests must be run manually
4. **Limited coverage**: Only 8 scenario files
5. **No performance tests**: No benchmarking infrastructure
6. **No stress tests**: No load/chaos testing

### Proposed Improvements
1. Update test framework to use correct binary name
2. Create 50+ comprehensive test scenarios
3. Add CI integration for automated testing
4. Add performance benchmarking suite
5. Add stress/chaos testing framework
6. Add integration test suite

## Test Prioritization

### Priority 1 (Critical - Test First)
1. Extended Thinking Phase (new feature)
2. Task Management System (new feature)
3. Memory System (new feature)
4. Slash command argument preservation (recent bugfix)
5. Core tool execution
6. Permission system basics

### Priority 2 (Important - Test Soon)
1. Agent Teams
2. Session Resume
3. Fast Mode
4. Background agents
5. MCP integration
6. Skills system

### Priority 3 (Nice to Have - Test Eventually)
1. Edge cases
2. Performance testing
3. Stress testing
4. UI/UX testing
5. Branding verification

## Test Execution Plan

### Phase 1: Setup and Baseline (Today)
1. Fix test framework binary name issue
2. Run existing 8 scenarios to establish baseline
3. Document all failures/bugs found
4. File issues for each bug

### Phase 2: Recent Feature Testing (Today)
1. Create scenarios for each recent feature (#331-338)
2. Execute scenarios
3. Document bugs
4. File issues

### Phase 3: Edge Case Testing (Today)
1. Create edge case scenarios
2. Execute scenarios
3. Document bugs
4. File issues

### Phase 4: Integration Testing (Today)
1. Create integration scenarios
2. Execute scenarios
3. Document bugs
4. File issues

### Phase 5: Bug Fixing (Parallel Workstreams)
1. Use DEFAULT_WORKFLOW for each bug
2. Create issues
3. Create branches
4. Fix bugs
5. Create PRs
6. Merge when ready

## Success Criteria

Testing is successful when:
- [ ] All 8 existing scenarios pass
- [ ] 50+ new scenarios created and passing
- [ ] All recent features (last 2 weeks) tested
- [ ] At least 20 edge cases tested
- [ ] All bugs found are documented with issues
- [ ] All critical bugs have PRs in progress
- [ ] Test framework improvements implemented
- [ ] CI integration complete

## Notes

- Focus on **finding bugs** not just confirming features work
- **Push the system to its limits** - that's where bugs hide
- **Test combinations** of features, not just individual features
- **Document everything** - every bug, every issue, every weird behavior
- **File issues immediately** - don't wait until the end
- **Use DEFAULT_WORKFLOW** for all bug fixes - it works!
