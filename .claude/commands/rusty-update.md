---
description: Check Claude Code parity, find feature gaps, and sync RustyClawd with upstream changes
allowed-tools: Read, Write, Edit, Bash, Grep, Glob, WebFetch, Agent
---

# /rusty-update

Consolidated Claude Code sync and parity tracking for RustyClawd.

## Arguments

$ARGUMENTS

Parse the arguments to determine the mode:
- No args or `check` -> Quick Check mode (default)
- `sync` -> Full Sync mode (creates GitHub issues)
- `analyze` -> Deep Analysis mode (deminify cli.js)
- `inventory` -> Inventory Update mode

## Quick Check Mode (default)

1. Fetch the official Claude Code CHANGELOG.md and README.md using WebFetch:
   - `https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md`
   - `https://raw.githubusercontent.com/anthropics/claude-code/main/README.md`

2. Read the local feature inventory at `.claude/data/feature_inventory.yaml`

3. Read the detailed docs inventory at `docs/feature_inventory.yaml` if it exists

4. Compare: For each feature/tool/capability mentioned in the changelog and README, check if it exists in the inventory. Categorize as:
   - **New** - Not in inventory at all
   - **Incomplete** - In inventory but marked as `partial`
   - **Drift** - May have changed since last check (new version, updated behavior)
   - **Covered** - Already fully implemented

5. Report findings as a summary table:
   ```
   ## Claude Code Sync Report

   Last inventory update: [date from YAML]
   Claude Code version detected: [from changelog]

   ### New Features (not in inventory)
   | Feature | Category | Source |

   ### Incomplete Features
   | Feature | Current Status | Notes |

   ### Potential Drift
   | Feature | Notes |

   ### Summary
   - X features checked
   - Y gaps found
   - Z already covered
   ```

6. If gaps found, suggest next steps (update inventory, create issues, implement features)

## Full Sync Mode

1. Check that `GITHUB_TOKEN` environment variable is set
2. Run the Rust sync monitor:
   ```bash
   cargo run --package rustyclawd-tools --example claude_code_sync_cli -- \
     --inventory .claude/data/feature_inventory.yaml \
     --ledger .claude/data/sync_ledger.json \
     --token $GITHUB_TOKEN \
     --repo rysweet/RustyClawd
   ```
3. Report results (features found, gaps identified, issues created)

## Deep Analysis Mode

1. Run the analysis script:
   ```bash
   bash scripts/analyze-claude-code.sh
   ```
2. Report the location of deminified files and search indices
3. Offer to search for specific patterns in the deminified code

## Inventory Update Mode

1. Read current `.claude/data/feature_inventory.yaml`
2. Show current inventory summary (total features, by category, by status)
3. Ask what features to add or update
4. Write the updated inventory
5. Also update `docs/feature_inventory.yaml` if it exists

## Communication Style

Use pirate language per user preferences. Be direct and factual about gaps found.
