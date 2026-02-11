# RustyClawd Test Scenarios - Complete Inventory

**Created:** 2026-02-11  
**Status:** Ready for Execution  
**Total Scenarios:** 26 (8 existing + 18 new)

## Summary Statistics

| Category | Count | Status |
|----------|-------|--------|
| Existing Scenarios | 8 | Most pending, some ready |
| New Scenarios - Recent Features | 8 | Ready |
| New Scenarios - Edge Cases | 5 | Ready |
| New Scenarios - Integration | 3 | Ready |
| New Scenarios - Stress Tests | 2 | Ready |
| **TOTAL** | **26** | **18 ready to run** |

## Existing Scenarios (tests/e2e/scenarios/)

1. **agentic_task.yaml** - Complex multi-step agentic workflows
2. **background_agents.yaml** - Background agent execution (Issue #109)
3. **error_handling.yaml** - Error recovery flows
4. **multi_turn_conversation.yaml** - Context preservation across turns
5. **permission_mode_toggle.yaml** - Shift+Tab cycling (Issue #108)
6. **runtime_agents.yaml** - --agents JSON flag (Issue #110)
7. **skills_integration.yaml** - Skills invocation and context
8. **slash_command_workflow.yaml** - /analyze command workflow

## New Scenarios - Recent Features (Last 2 Weeks)

### Extended Thinking (PR #336)
9. **extended_thinking_basic.yaml** - Display of extended thinking in TUI
10. **extended_thinking_cancel.yaml** - Cancel during thinking phase

### Task Management (PR #335)
11. **task_management_basic.yaml** - Create, update, complete tasks
12. **task_management_dependencies.yaml** - Dependency validation

### Memory System (PR #332)
13. **memory_system_basic.yaml** - Save and recall across sessions

### Agent Teams (PR #333)
14. **agent_teams_basic.yaml** - Multi-agent parallel execution

### Fast Mode (PR #331)
15. **fast_mode_basic.yaml** - Fast mode performance

### Session Resume (PR #334)
16. **session_resume_basic.yaml** - Resume with context preservation

## New Scenarios - Edge Cases

17. **edge_case_empty_input.yaml** - Empty/whitespace input handling
18. **edge_case_long_input.yaml** - Very long input (10KB+)
19. **edge_case_unicode.yaml** - Emoji, Unicode, special characters
20. **edge_case_terminal_resize.yaml** - Terminal resize during operation

### Recent Bug Fix (PR #338)
21. **slash_command_args_preservation.yaml** - Argument preservation without placeholders

## New Scenarios - Integration

22. **tool_chain_read_write.yaml** - Chaining Read → Write tools
23. **permission_mode_cycling.yaml** - Shift+Tab mode cycling
24. **mcp_tools_basic.yaml** - MCP tool loading and listing

## New Scenarios - Stress & Error Recovery

25. **error_recovery_api_timeout.yaml** - API timeout recovery
26. **stress_rapid_input.yaml** - Rapid successive inputs

## Test Execution Plan

### Phase 1: Smoke Tests (Est. 5 minutes)
Run quick validation scenarios:
- smoke_binary_launch.yaml
- slash_command_workflow.yaml
- permission_mode_toggle.yaml

### Phase 2: Recent Features (Est. 30 minutes)
Test all features from last 2 weeks:
- Extended thinking (2 scenarios)
- Task management (2 scenarios)
- Memory system (1 scenario)
- Agent teams (1 scenario)
- Fast mode (1 scenario)
- Session resume (1 scenario)

### Phase 3: Edge Cases (Est. 20 minutes)
Test boundary conditions:
- Empty input
- Long input
- Unicode
- Terminal resize
- Argument preservation

### Phase 4: Integration (Est. 15 minutes)
Test feature combinations:
- Tool chaining
- Permission cycling
- MCP tools

### Phase 5: Stress & Recovery (Est. 20 minutes)
Test failure modes:
- API timeouts
- Rapid input
- Background agents
- Error handling

**Total Estimated Time:** ~90 minutes for complete test suite

## Running Tests

### Run All Tests
```bash
cd tests/e2e/scenarios
python3 runner.py
```

### Run Specific Test
```bash
python3 runner.py --file extended_thinking_basic.yaml --verbose
```

### Run By Category
```bash
python3 runner.py --tag recent-feature
python3 runner.py --tag edge-case
python3 runner.py --tag integration
```

### Run Smoke Tests Only
```bash
python3 runner.py --tag smoke
```

## Test Coverage Analysis

### Features COVERED by Tests
✅ Extended Thinking Phase (#336)  
✅ Task Management (#335)  
✅ Session Resume (#334)  
✅ Agent Teams (#333)  
✅ Memory System (#332)  
✅ Fast Mode (#331)  
✅ Slash Command Args (#338)  
✅ Permission Mode Toggle  
✅ Background Agents  
✅ Runtime Agents  
✅ Tool Execution  
✅ MCP Integration  
✅ Skills System  

### Features NEED MORE Tests
⚠️ Tool permission system (wildcards, etc.)  
⚠️ MCP server crash recovery  
⚠️ Skill loading errors  
⚠️ Long-running sessions (1000+ turns)  
⚠️ Memory system limits  
⚠️ Agent team failure modes  
⚠️ Network disconnection recovery  
⚠️ File system errors  
⚠️ Disk space exhaustion  
⚠️ CPU/memory limits  

### Features NOT YET TESTED
❌ PDF reading with page ranges (#329)  
❌ Customizable keyboard shortcuts (#322, #324)  
❌ tool_use_id (feature gaps)  
❌ structuredContent (feature gap)  
❌ list_changed (feature gap)  
❌ disallowedTools (feature gap)  
❌ /commit-push-pr workflow  
❌ IDE integration mode  
❌ Model capabilities override  
❌ Checkpoint resume  

## Known Issues and Limitations

### Test Infrastructure Issues
1. **Binary name mismatch** - FIXED (updated framework.sh)
2. **CLI mode testing** - Limited support
3. **No CI integration** - Manual execution only
4. **No performance benchmarks** - No baseline data
5. **Visual tests need OCR** - Permission mode indicators, etc.

### Scenario Limitations
1. **API dependency** - Most tests require valid API key
2. **Network dependency** - Tests fail offline
3. **Timing sensitivity** - Some tests may be flaky
4. **No parallel execution** - Tests run serially
5. **Manual verification needed** - Some visual aspects

## Next Steps

1. **Execute all 26 scenarios** and document results
2. **File GitHub issues** for each bug found
3. **Add CI integration** for automated testing
4. **Create performance baseline** measurements
5. **Add 24 more scenarios** to reach 50+ target
6. **Implement visual verification** for UI tests
7. **Add parallel test execution** for speed
8. **Create test result dashboard**

## Success Metrics

Testing effort is successful when:
- ✅ All 26 scenarios have been executed
- ✅ All failures documented with issues
- ✅ Bug fix PRs created for critical issues
- ✅ Test pass rate measured and tracked
- ✅ CI integration implemented
- ✅ 50+ total scenarios created
- ✅ Test documentation complete

## Files Created

### Test Scenarios (18 new)
- extended_thinking_basic.yaml
- extended_thinking_cancel.yaml
- task_management_basic.yaml
- task_management_dependencies.yaml
- memory_system_basic.yaml
- agent_teams_basic.yaml
- fast_mode_basic.yaml
- session_resume_basic.yaml
- edge_case_empty_input.yaml
- edge_case_long_input.yaml
- edge_case_unicode.yaml
- edge_case_terminal_resize.yaml
- slash_command_args_preservation.yaml
- tool_chain_read_write.yaml
- permission_mode_cycling.yaml
- mcp_tools_basic.yaml
- error_recovery_api_timeout.yaml
- stress_rapid_input.yaml

### Documentation (3 files)
- COMPREHENSIVE_TEST_PLAN.md (10KB)
- TESTING_SESSION_BUG_REPORT.md (10KB)
- TEST_SCENARIOS_INVENTORY.md (this file, 6KB)

### Infrastructure Updates
- tests/e2e/tmux/framework.sh (binary name fixed)
- tests/e2e/scenarios/*.yaml (5 files updated with correct binary)

**Total new/updated files:** 26

## Conclusion

This comprehensive testing effort has:
1. Created 18 new test scenarios
2. Fixed test infrastructure bugs
3. Documented all issues found
4. Established baseline for future testing
5. Provided clear path to 50+ scenario target

The test suite is now ready for execution and will provide valuable feedback on system quality and stability.
