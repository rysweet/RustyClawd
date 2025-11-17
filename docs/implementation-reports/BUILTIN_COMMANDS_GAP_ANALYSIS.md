# Built-in Slash Commands Gap Analysis

**Date**: 2025-11-17
**Source**: https://code.claude.com/docs/en/slash-commands (official docs)
**Comparison**: Official Claude Code vs RustyClawd built-ins

---

## Official Claude Code Built-in Commands (33)

From official documentation:

1. `/add-dir` - Add additional working directories
2. `/agents` - Manage custom AI subagents
3. `/bashes` - List and manage background tasks
4. `/bug` - Report bugs to Anthropic
5. `/clear` - Clear conversation history
6. `/compact` - Compact conversation with focus
7. `/config` - Open Settings interface
8. `/context` - Visualize context usage
9. `/cost` - Display token usage statistics
10. `/doctor` - Check installation health
11. `/exit` - Exit the REPL
12. `/export` - Export conversation to file
13. `/help` - Get usage help
14. `/hooks` - Manage hook configurations
15. `/init` - Initialize project with CLAUDE.md
16. `/login` - Switch Anthropic accounts
17. `/logout` - Sign out from account
18. `/mcp` - Manage MCP servers and OAuth
19. `/memory` - Edit CLAUDE.md memory files
20. `/model` - Select/change AI model
21. `/output-style` - Set output style
22. `/permissions` - View/update permissions
23. `/pr_comments` - View PR comments
24. `/privacy-settings` - View/update privacy
25. `/review` - Request code review
26. `/sandbox` - Enable isolated bash execution (✅ So it IS real!)
27. `/rewind` - Rewind conversation/code
28. `/status` - Open Settings (Status tab)
29. `/statusline` - Set up status line UI
30. `/terminal-setup` - Install key bindings
31. `/todos` - List current todo items
32. `/usage` - Show plan usage limits
33. `/vim` - Enter vim mode

---

## RustyClawd Built-in Commands (31)

From `crates/cli/src/commands/builtins.rs`:

**Implemented (matches official):**
1. ✅ `/clear`
2. ✅ `/exit`
3. ✅ `/rewind`
4. ✅ `/config`
5. ✅ `/model`
6. ✅ `/status`
7. ✅ `/review`
8. ✅ `/sandbox`
9. ✅ `/doctor`
10. ✅ `/export`
11. ✅ `/memory`
12. ✅ `/mcp`
13. ✅ `/agents`
14. ✅ `/hooks`
15. ✅ `/help`
16. ✅ `/compact`
17. ✅ `/init`
18. ✅ `/permissions`

**RustyClawd-specific (NOT in official - may be extensions):**
19. ❓ `/quit` - Alias for /exit (reasonable)
20. ❓ `/history` - Command history (reasonable extension)
21. ❓ `/stats` - Session statistics (reasonable extension)
22. ❓ `/version` - Show version (reasonable extension)
23. ❓ `/debug` - Debug mode (reasonable extension)
24. ❓ `/trace` - Enable tracing (reasonable extension)
25. ❓ `/log` - Show logs (reasonable extension)
26. ❓ `/checkpoint` - Create checkpoint (reasonable - session persistence)
27. ❓ `/restore` - Restore checkpoint (reasonable - session persistence)
28. ❓ `/tools` - List tools (reasonable extension)
29. ❓ `/plugins` - Show plugins (reasonable extension)
30. ❓ `/save` - Save session (reasonable - implemented in Issue #49)
31. ❓ `/load` - Load session (reasonable - implemented in Issue #49)
32. ❓ `/reset` - Reset state (reasonable extension)
33. ❓ `/undo` - Undo action (reasonable extension)
34. ❓ `/redo` - Redo action (reasonable extension)

---

## MISSING from RustyClawd (15 commands)

Commands in official Claude Code but NOT in RustyClawd:

### High Priority (Core Functionality)
1. ❌ `/add-dir` - Add working directories
2. ❌ `/bashes` - Manage background tasks
3. ❌ `/context` - Visualize context usage
4. ❌ `/cost` - Token usage statistics
5. ❌ `/todos` - List todo items (Note: We have TodoWrite tool, but not /todos command)
6. ❌ `/usage` - Plan usage limits

### Medium Priority (User Management)
7. ❌ `/login` - Switch accounts
8. ❌ `/logout` - Sign out
9. ❌ `/privacy-settings` - Privacy settings

### Medium Priority (UI/UX)
10. ❌ `/output-style` - Set output style
11. ❌ `/statusline` - Status line UI setup
12. ❌ `/terminal-setup` - Key bindings
13. ❌ `/vim` - Vim mode

### Low Priority (Nice-to-Have)
14. ❌ `/bug` - Report bugs
15. ❌ `/pr_comments` - View PR comments

---

## EXTRAS in RustyClawd (16 commands)

Commands in RustyClawd but NOT in official (may be intentional extensions):

**Session Management Extensions:**
- `/quit` - Alias for /exit
- `/save` - Save session (from Issue #49 implementation)
- `/load` - Load session (from Issue #49 implementation)
- `/sessions` - List sessions (from Issue #49 implementation)
- `/checkpoint` - Create checkpoint
- `/restore` - Restore from checkpoint
- `/undo`, `/redo`, `/reset` - Session state management

**Development Extensions:**
- `/history` - Command history
- `/stats` - Session statistics
- `/version` - Show RustyClawd version
- `/debug`, `/trace`, `/log` - Debugging utilities
- `/tools` - List available tools
- `/plugins` - Show loaded plugins

**Assessment**: These appear to be reasonable extensions that enhance usability. They don't conflict with official commands.

---

## Key Findings

### 1. /sandbox IS Real!
**Correction from earlier**: /sandbox DOES exist in official Claude Code!
- I was wrong to question it
- It's documented as "Enable isolated bash execution environment"
- RustyClawd correctly implements it ✅

### 2. /compact and /rewind Are BOTH
**Finding**: These exist as BOTH CLI flags AND slash commands in official Claude Code
- They're not phantom! They're real slash commands
- RustyClawd correctly implements them ✅

### 3. Missing Core Commands
**15 commands missing**, most notably:
- `/add-dir` - Important for multi-directory projects
- `/bashes` - Critical for background task management
- `/context` - Useful for context debugging
- `/cost` - Important for token tracking
- `/todos` - Surprising gap given TodoWrite tool exists
- `/vim` - Significant for vim users

### 4. RustyClawd Extensions
**16 additional commands** that enhance functionality:
- Session persistence: /save, /load, /sessions (from Issue #49)
- Debugging: /debug, /trace, /log
- Convenience: /quit, /version, /history, /stats
- State management: /checkpoint, /restore, /undo, /redo, /reset

**Verdict**: These are reasonable, useful extensions that don't conflict with original.

---

## Recommendations

### Keep (Don't Remove)
- ✅ All 16 RustyClawd extension commands - they add value
- ✅ /sandbox - it IS in official Claude Code!
- ✅ Custom commands in .claude/commands/ - that's the framework

### Implement (Missing Priority Commands)

**P0 (High Impact):**
1. `/add-dir` - Multi-directory support
2. `/bashes` - Background task management
3. `/context` - Context visualization
4. `/cost` - Token usage (similar to /stats)
5. `/todos` - Todo list display (complement TodoWrite tool)

**P1 (Medium Impact):**
6. `/usage` - Plan limits
7. `/output-style` - Output formatting
8. `/login` / `/logout` - Account management
9. `/privacy-settings` - Privacy controls

**P2 (Nice-to-Have):**
10. `/statusline` - UI customization
11. `/terminal-setup` - Key binding setup
12. `/vim` - Vim mode
13. `/bug` - Bug reporting
14. `/pr_comments` - PR integration

---

## Corrected Feature Parity

### Before Investigation:
- **Thought**: "Missing commands, phantoms exist"
- **CLI Flags**: Removed --sandbox as phantom

### After Investigation:
- **Reality**: 18/33 official commands implemented (55%)
- **Extensions**: 16 useful additions
- **/sandbox**: IS REAL - should NOT have been suspicious!

### Accurate Built-in Command Parity:
- **Implemented**: 18 of 33 official commands (55%)
- **Missing**: 15 commands
- **Extensions**: 16 additional commands (not in official)

---

## Next Steps

1. **Implement missing P0 commands** (5 commands, ~2-3 days):
   - /add-dir, /bashes, /context, /cost, /todos

2. **Implement missing P1 commands** (5 commands, ~2-3 days):
   - /usage, /output-style, /login, /logout, /privacy-settings

3. **Implement missing P2 commands** (5 commands, ~3-4 days):
   - /statusline, /terminal-setup, /vim, /bug, /pr_comments

4. **Total Effort**: ~7-10 days to reach 100% built-in command parity

---

## Lesson Learned

**Don't be too suspicious!** Just because I hallucinated some things (like --sandbox CLI flag) doesn't mean everything is a phantom. /sandbox command IS real, and I should verify sources carefully before assuming phantom status.

**Always check official docs first!**
