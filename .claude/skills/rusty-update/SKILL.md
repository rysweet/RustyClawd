---
name: rusty-update
version: 1.0.0
description: |
  Consolidated Claude Code sync and parity tracking for RustyClawd.
  Fetches latest Claude Code features from official sources, compares against
  the local feature inventory, identifies gaps, and optionally creates GitHub issues.
  Also supports deep analysis by deminifying the installed Claude Code cli.js.
auto_activates:
  - "rusty update"
  - "sync with claude code"
  - "check claude code parity"
  - "feature parity"
  - "what's new in claude code"
  - "claude code changes"
  - "catch up with claude code"
  - "feature gaps"
  - "parity check"
  - "sync monitor"
priority_score: 45.0
evaluation_criteria:
  frequency: MEDIUM
  impact: HIGH
  complexity: MEDIUM
  reusability: HIGH
  philosophy_alignment: HIGH
  uniqueness: HIGH
invokes:
  - type: tool
    name: WebFetch
  - type: tool
    name: Bash
  - type: tool
    name: Read
  - type: tool
    name: Write
  - type: tool
    name: Grep
  - type: tool
    name: Glob
dependencies:
  tools:
    - Read
    - Write
    - Bash
    - Grep
    - Glob
    - WebFetch
  external:
    - "cargo (for running Rust sync monitor)"
    - "git"
    - "curl"
  data_files:
    - ".claude/data/feature_inventory.yaml"
    - ".claude/data/sync_ledger.json"
    - "docs/feature_inventory.yaml"
  rust_modules:
    - "crates/tools/src/claude_code_sync/"
philosophy:
  - principle: Ruthless Simplicity
    application: Single skill consolidates three separate tools into one workflow
  - principle: Zero-BS Implementation
    application: Real HTTP fetches and GitHub API calls, no mocked data
  - principle: Modular Design
    application: Rust sync monitor is the engine; skill is the interface
maturity: production
maturity_reason: |
  - Rust SyncMonitor module fully implemented with unit tests
  - CI workflow running weekly (.github/workflows/claude-code-sync.yml)
  - Feature inventory actively maintained with 58+ features tracked
  - Sync ledger preventing duplicate issue creation
---

# /rusty-update

Consolidated Claude Code sync and parity tracking skill for RustyClawd.

## What This Skill Does

This skill consolidates three previously separate tools into one unified workflow:

1. **Feature Discovery** - Fetches Claude Code's CHANGELOG.md and README.md from GitHub
2. **Gap Analysis** - Compares discovered features against `.claude/data/feature_inventory.yaml`
3. **Reporting** - Summarizes what's new, what's missing, and what's drifted
4. **Issue Creation** (optional) - Creates GitHub issues for gaps via the sync ledger

Additionally supports:
5. **Deep Analysis** - Deminifies installed Claude Code `cli.js` for pattern research

## Modes

### Quick Check (default)
Fetches Claude Code sources and compares against inventory. No side effects.

```
/rusty-update
/rusty-update check
```

### Full Sync (creates GitHub issues)
Runs the Rust SyncMonitor which also creates GitHub issues for gaps.

```
/rusty-update sync
```

Requires: `GITHUB_TOKEN` environment variable.

### Deep Analysis
Deminifies the installed Claude Code cli.js and creates searchable indices.

```
/rusty-update analyze
```

Requires: `prettier` and `js-beautify` (auto-installed if missing).

### Inventory Update
Interactively update the feature inventory after implementing new features.

```
/rusty-update inventory
```

## Underlying Tools (Consolidated)

| Previous Tool | Location | Status | Merged Into |
|---|---|---|---|
| `claude_code_sync_cli` | `crates/tools/examples/` | Working | `sync` mode |
| `analyze-claude-code.sh` | `scripts/` | Working | `analyze` mode |
| `fetch_claude_features.rs` | `scripts/` | Redundant | `check` mode (superseded) |
| `claude-code-sync.yml` | `.github/workflows/` | Working | Unchanged (CI) |

## Data Files

- **Feature Inventory**: `.claude/data/feature_inventory.yaml` - What RustyClawd implements
- **Sync Ledger**: `.claude/data/sync_ledger.json` - Issue deduplication tracking
- **Docs Inventory**: `docs/feature_inventory.yaml` - Detailed docs version with test evidence

## Architecture

```
/rusty-update (skill)
    |
    +-- check: WebFetch CHANGELOG.md + README.md -> compare with inventory -> report
    |
    +-- sync:  cargo run claude_code_sync_cli -> full sync with issue creation
    |
    +-- analyze: scripts/analyze-claude-code.sh -> deminify + index cli.js
    |
    +-- inventory: Interactive inventory update workflow
```

## Execution

When this skill activates, follow this procedure:

### For `check` mode (default):

1. **Fetch** the official Claude Code CHANGELOG.md and README.md:
   - `https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md`
   - `https://raw.githubusercontent.com/anthropics/claude-code/main/README.md`

2. **Read** the local feature inventory:
   - `.claude/data/feature_inventory.yaml`

3. **Compare** features found in the changelog against the inventory:
   - New features not in inventory = **Missing**
   - Features marked partial = **Incomplete**
   - Features that may have changed = **Drift**

4. **Report** findings as a summary table with:
   - New Claude Code features since last check
   - Gaps (missing/incomplete/drift)
   - Recommended actions

### For `sync` mode:

1. Verify `GITHUB_TOKEN` is set
2. Run: `cargo run --package rustyclawd-tools --example claude_code_sync_cli -- --inventory .claude/data/feature_inventory.yaml --ledger .claude/data/sync_ledger.json --token $GITHUB_TOKEN --repo rysweet/RustyClawd`
3. Report created issues

### For `analyze` mode:

1. Run: `bash scripts/analyze-claude-code.sh`
2. Report location of deminified files and indices

### For `inventory` mode:

1. Read current `.claude/data/feature_inventory.yaml`
2. Ask what features to add/update
3. Write updated inventory
4. Optionally update `docs/feature_inventory.yaml` too
