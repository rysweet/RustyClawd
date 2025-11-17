# Backend Integration Report: Rate Limits & Background Shells

## Executive Summary

Successfully integrated real backend functionality for two critical commands:
1. **/usage** - Now displays live rate limit data from Anthropic API responses
2. **/bashes** - Now shows actual background shell processes with real-time status

Both commands now provide functional, real-time data instead of placeholder information.

---

## Task 1: /usage Command - Real Rate Limit Data

### Implementation Overview

Added complete rate limit tracking by extracting HTTP headers from API responses and storing them in session statistics.

### Changes Made

#### 1. Extended `SessionStats` Structure (`crates/cli/src/session.rs`)

Added new `RateLimitData` struct to track:
- Request limits (per minute)
- Token limits (per day)
- Remaining counts for both
- Reset timestamps
- Last update time

```rust
pub struct RateLimitData {
    pub requests_limit: Option<u32>,
    pub requests_remaining: Option<u32>,
    pub requests_reset: Option<u64>,
    pub tokens_limit: Option<u64>,
    pub tokens_remaining: Option<u64>,
    pub tokens_reset: Option<u64>,
    pub last_updated: Option<DateTime<Utc>>,
}
```

#### 2. Header Extraction (`crates/cli/src/session.rs`)

Implemented `HeaderMapLike` trait to handle HTTP header extraction:
- Generic interface for different HeaderMap versions
- Parses Anthropic rate limit headers: `anthropic-ratelimit-*`
- Calculates percentage used for visual progress bars

#### 3. API Response Integration (`crates/cli/src/interactive.rs`)

Modified `stream_single_turn_with_messages` to:
- Make direct HTTP requests to capture headers
- Extract rate limit headers before consuming response body
- Update session stats with live data on every API call

#### 4. Updated /usage Command Handler (`crates/cli/src/interactive.rs`)

Created `handle_usage_command` method that:
- Displays real rate limit data when available
- Shows helpful message when no data yet (before first API call)
- Renders visual progress bars for requests and tokens
- Displays last update timestamp

### Example Output

```
API Usage & Rate Limits:

Rate Limits (Per Minute):
- Requests:    153 / 1000   used (15%)
- Remaining:    847 requests

Token Limits (Per Day):
- Tokens:      1579000 / 5000000 used (31%)
- Remaining:   3421000 tokens

Visual Progress:
Requests: [=======                                           ] 15%
Tokens:   [===============                                   ] 31%

Last updated: 2025-11-17 14:23:45 UTC
```

### Dependencies Added

- `secrecy = "0.8"` - For secure API key handling
- `http = "1.3"` - For HeaderMap compatibility with reqwest

---

## Task 2: /bashes Command - Real Background Shell Tracking

### Implementation Overview

Integrated with existing `ProcessRegistry` from `rustyclawd-tools` to display actual running background shell processes.

### Changes Made

#### 1. Added /bashes Handler (`crates/cli/src/interactive.rs`)

Created `handle_bashes_command` async method that:
- Queries global process registry for active shells
- Retrieves status for each shell (Running, Completed, Failed)
- Displays shell IDs with current status
- Provides usage instructions for BashOutput and KillShell tools

#### 2. Integration with ProcessRegistry

Leverages existing infrastructure:
- `rustyclawd_tools::process_registry::global_registry()` - Shared state
- `ProcessStatus` enum - Running, Completed(exit_code), Failed(error)
- Thread-safe access via Arc<Mutex<HashMap>>

### Example Output

**No shells running:**
```
Background Bash Shells:

No background shells currently running.

Tips:
- Background shells are created using Bash tool with run_in_background: true
- Use BashOutput tool to read shell output
- Use KillShell tool to terminate shells
```

**With active shells:**
```
Background Bash Shells (3):

  shell_a3f2b9c1 - Running
  shell_d8e4f7a2 - Completed (success)
  shell_b1c5d9e3 - Running

Commands:
- Use BashOutput tool with bash_id to read output
- Use KillShell tool with shell_id to terminate

Example: Ask Claude to check output from a specific shell ID
```

---

## Technical Details

### Core Client Extension (`crates/core/src/client/mod.rs`)

Added public getter methods to `Client` for custom request handling:
```rust
pub fn api_url(&self) -> &str
pub fn api_version(&self) -> &str
pub fn http_client(&self) -> &HttpClient
pub fn config(&self) -> &Config
```

These enable CLI to make direct HTTP requests while capturing headers.

### Architecture Decisions

**Rate Limit Tracking:**
- Captures headers on every streaming API call
- Updates are automatic and transparent
- Data persists in session stats for entire session duration
- Uses generic `HeaderMapLike` trait to handle http crate version mismatches

**Background Shell Management:**
- Leverages existing ProcessRegistry infrastructure
- No new state management needed
- Thread-safe via Arc<Mutex<>>
- Integrates with existing Bash, BashOutput, and KillShell tools

---

## Testing & Verification

### Build Status
✓ Compiled successfully with all dependencies
✓ No warnings or errors
✓ Release profile: optimized build

### Integration Points
✓ Rate limit headers extracted from streaming responses
✓ Session stats updated automatically on each API call
✓ Process registry query works with existing tool system
✓ Both commands accessible via TUI

### Backward Compatibility
✓ Existing functionality preserved
✓ No breaking changes to public APIs
✓ Graceful handling when no data available

---

## Files Modified

### Core Changes
1. `/home/azureuser/src/RustyClawd/crates/cli/src/session.rs`
   - Added `RateLimitData` struct
   - Implemented `HeaderMapLike` trait
   - Extended `SessionStats` with rate_limits field

2. `/home/azureuser/src/RustyClawd/crates/cli/src/interactive.rs`
   - Modified `stream_single_turn_with_messages` for header extraction
   - Added `handle_usage_command` method
   - Added `handle_bashes_command` async method
   - Wired up /usage and /bashes command routing

3. `/home/azureuser/src/RustyClawd/crates/core/src/client/mod.rs`
   - Added public getter methods for Client configuration
   - Enables custom request handling in CLI layer

4. `/home/azureuser/src/RustyClawd/crates/cli/Cargo.toml`
   - Added `secrecy = "0.8"`
   - Added `http = "1.3"`

---

## Future Enhancements

### Potential Improvements
1. **Rate Limit Warnings**: Alert user when approaching limits
2. **Shell Management UI**: Interactive shell selection/termination
3. **Historical Tracking**: Store rate limit trends over time
4. **Cost Estimation**: Combine with /cost command for projected expenses

### Known Limitations
1. Rate limit data only available after first API call
2. No persistence of rate limit data between sessions
3. Shell listing doesn't show command details (by design for security)

---

## Conclusion

Both critical backend integrations are now fully functional:

**✓ /usage** - Provides real-time rate limit monitoring from live API responses
**✓ /bashes** - Shows actual background shell processes with current status

The implementation is production-ready, fully integrated with existing systems, and maintains backward compatibility. All code follows Rust best practices with proper error handling, type safety, and clean architecture.

---

**Implementation Date**: 2025-11-17
**Build Status**: ✓ Success (release profile)
**Test Coverage**: Manual verification complete
