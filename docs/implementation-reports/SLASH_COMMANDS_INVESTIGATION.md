# Slash Commands Investigation Report

**Date**: 2025-11-17
**Question**: "What / commands are missing?"
**Status**: Investigation Complete (with limitations)

---

## Investigation Limitation

**Network Restriction**: Cannot access official Claude Code documentation at code.claude.com due to network/enterprise security policies.

**Impact**: Cannot definitively compare RustyClawd commands against official Claude Code commands.

**What We CAN Confirm**: Complete inventory of RustyClawd's slash commands (verified from codebase).

---

## RustyClawd Slash Commands: Complete Inventory

### Total: 67 Commands

**Built-in Commands**: 35
**Custom Commands**: 32 (8 top-level + 24 amplihack)

---

## Built-in Commands (35)

**Location**: `crates/cli/src/commands/builtins.rs`

### Session Management (4)
1. `/clear` - Clear conversation history
2. `/exit` - Exit session
3. `/quit` - Exit session (alias)
4. `/rewind` - Rewind to previous state

### Configuration (3)
5. `/config` - View/modify configuration
6. `/model` - Switch/view model
7. `/status` - Show system status

### Development (3)
8. `/review` - Review code/PR
9. `/sandbox` - Enable sandbox mode
10. `/doctor` - Run diagnostics

### File Operations (2)
11. `/export` - Export conversation
12. `/memory` - Memory usage stats

### Integration (3)
13. `/mcp` - MCP server integration
14. `/agents` - List agents
15. `/hooks` - Show hooks

### Information (3)
16. `/help` - Show help
17. `/history` - Command history
18. `/stats` - Session statistics

### Tools (2)
19. `/tools` - List available tools
20. `/plugins` - Show loaded plugins

### Session State (5)
21. `/save` - Save session
22. `/load` - Load session
23. `/reset` - Reset state
24. `/undo` - Undo action
25. `/redo` - Redo action

### Additional (10)
26. `/compact` - Compact history
27. `/init` - Initialize config
28. `/version` - Show version
29. `/permissions` - Display permissions
30. `/debug` - Debug mode
31. `/trace` - Enable tracing
32. `/log` - Show logs
33. `/checkpoint` - Create checkpoint
34. `/restore` - Restore checkpoint
35. `/sessions` - List sessions (added in this PR)

---

## Custom Commands (32)

### Top-Level Commands (8)

**Core Development:**
1. `/analyze` - Code/system analysis
2. `/debug` - Debug mode with detailed logging
3. `/ultrathink` - Deep analysis mode

**MCP Integration:**
4. `/mcp-list` - List MCP servers
5. `/mcp-start` - Start MCP server
6. `/mcp-stop` - Stop MCP server
7. `/mcp-status` - MCP server status
8. `/mcp-tools` - List MCP tools

### Amplihack Commands (24)

**Core Framework:**
9. `/amplihack:customize` - Preference management
10. `/amplihack:skill-builder` - Build new skills
11. `/amplihack:knowledge-builder` - Build knowledge bases
12. `/amplihack:ultrathink` - Enhanced deep analysis

**Analysis:**
13. `/amplihack:analyze` - Deep code analysis
14. `/amplihack:socratic` - Socratic questioning
15. `/amplihack:expert-panel` - Multi-agent panel

**Code Improvement:**
16. `/amplihack:fix` - Issue resolution
17. `/amplihack:improve` - Improvement suggestions
18. `/amplihack:ingest-code` - Codebase documentation
19. `/amplihack:modular-build` - Modular system building

**Workflow:**
20. `/amplihack:cascade` - Cascading workflow
21. `/amplihack:debate` - Multi-perspective debate
22. `/amplihack:n-version` - N-version programming
23. `/amplihack:reflect` - Session reflection
24. `/amplihack:auto` - Automated execution

**Utilities:**
25. `/amplihack:transcripts` - Transcript management
26. `/amplihack:xpia` - Cross-project assistant
27. `/amplihack:lock` - Resource locking
28. `/amplihack:unlock` - Resource unlocking
29. `/amplihack:install` - Install framework
30. `/amplihack:uninstall` - Uninstall framework

**Documentation:**
31. `/amplihack:SKILL_BUILDER_EXAMPLES` - Skill examples
32. `/amplihack:reflection` - Reflection system (legacy)

---

## What We Know vs. Don't Know

### ✅ Confirmed (from codebase analysis):
- RustyClawd has 67 total slash commands
- All 35 built-in commands exist in builtins.rs
- All 32 custom commands exist as .md files
- Command system is fully functional with 63 passing tests

### ❌ Cannot Confirm (network access blocked):
- Which commands exist in official Claude Code
- Which RustyClawd commands are missing from original
- Which RustyClawd commands are extensions/phantoms
- Official command syntax and behavior

---

## Recommendations

### Immediate Action: Manual Verification

Since I cannot access official Claude Code docs, YOU should:

1. **Check Official Docs Manually**:
   - Visit: https://code.claude.com/docs/en/slash-commands
   - Or run: `claude /help` in official Claude Code
   - Compare output with RustyClawd's `/help`

2. **Create Comparison Matrix**:
   - List official commands
   - Mark which are in RustyClawd
   - Identify missing commands
   - Identify phantom commands

3. **Prioritize Missing Commands**:
   - P0: Commands users expect
   - P1: Nice-to-have features
   - P2: Rarely-used commands

### Likely Scenarios

**Scenario A: RustyClawd Has MORE Commands**
- Amplihack framework adds 24 commands not in original
- MCP commands may be RustyClawd-specific
- Some built-ins may be extensions

**Scenario B: Missing Core Commands**
- Official Claude Code may have commands RustyClawd lacks
- Check especially: session management, configuration, tool control

**Scenario C: Different Names, Same Function**
- Commands may exist but with different names
- Example: `/clear` vs `/reset` for same function

---

## Self-Audit: Suspicious Commands

Based on my investigation of --sandbox (which was phantom), these commands might NOT be in original:

**High Suspicion** (likely RustyClawd-specific):
- `/sandbox` - Similar to --sandbox phantom
- `/review` - May be amplihack-specific
- `/doctor` - Sounds like RustyClawd addition
- `/compact` - Mentioned in phantom research
- `/rewind` - Mentioned in phantom research
- All `/amplihack:*` commands - Definitely RustyClawd-specific
- All `/mcp-*` commands - May be RustyClawd-specific

**Medium Suspicion** (might be in original):
- `/save`, `/load`, `/sessions` - Common session commands
- `/checkpoint`, `/restore` - May match original's session handling
- `/tools`, `/plugins` - Likely in original for tool management

**Low Suspicion** (almost certainly in original):
- `/help`, `/clear`, `/exit`, `/quit` - Universal commands
- `/history`, `/stats` - Common information commands
- `/config`, `/model`, `/status` - Configuration commands

---

## Action Items for User

1. **Verify Against Official Claude Code**:
   ```bash
   # In official Claude Code
   claude /help

   # In RustyClawd
   rusty /help

   # Compare the two lists
   ```

2. **Create Definitive List**:
   - Official commands (from Claude Code)
   - RustyClawd commands (from this report)
   - Missing commands (Original - RustyClawd)
   - Extension commands (RustyClawd - Original)

3. **Decision on Extensions**:
   - Keep amplihack commands? (They're RustyClawd-specific features)
   - Remove phantom built-ins? (Like we did with --sandbox)
   - Document divergences clearly?

---

## Files for Reference

**This Investigation**:
- `SLASH_COMMANDS_INVESTIGATION.md` (this file)

**Command Implementation**:
- `crates/cli/src/commands/builtins.rs` - 35 built-in commands
- `.claude/commands/*.md` - 8 top-level custom commands
- `.claude/commands/amplihack/*.md` - 24 amplihack commands

**Tests**:
- `crates/cli/src/commands/mod.rs` - 63 passing tests

---

## Conclusion

**Confirmed**: RustyClawd has 67 slash commands implemented
**Unknown**: Which of these are authentic to Claude Code vs phantoms/extensions
**Limitation**: Cannot access official docs to verify

**Next Steps**: Manual verification against official Claude Code required

---

**Investigation Status**: Complete (within network constraints)
**Confidence Level**: High for RustyClawd inventory, Unknown for comparison
