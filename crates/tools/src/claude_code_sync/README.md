# Claude Code Sync Monitor

Automated monitoring system that tracks Claude Code features and creates GitHub issues for parity gaps.

## Purpose

Prevents misunderstandings about Claude Code feature coverage by automatically:
1. Fetching Claude Code feature announcements from changelog/docs
2. Comparing with RustyClawd's feature inventory
3. Creating GitHub issues for missing or incomplete features
4. Maintaining a ledger to prevent duplicate issues

## Architecture

Three self-contained "bricks" following the philosophy of ruthless simplicity:

### 1. Feature Discovery (`feature_discovery.rs`)
- **Responsibility**: Fetch and parse Claude Code features from documentation
- **Input**: HTTP requests to Claude Code docs/changelog
- **Output**: List of `ClaudeFeature` structs
- **Stud (Public API)**:
  ```rust
  pub struct FeatureDiscovery {
      pub fn new() -> Self
      pub async fn fetch_features(&self) -> Result<Vec<ClaudeFeature>>
  }
  ```

### 2. Feature Mapping (`feature_mapping.rs`)
- **Responsibility**: Compare Claude Code features with RustyClawd inventory
- **Input**: Claude features + feature inventory YAML file
- **Output**: List of `FeatureGap` structs
- **Stud (Public API)**:
  ```rust
  pub struct FeatureMapper {
      pub fn new(inventory_path: impl Into<String>) -> Self
      pub async fn find_gaps(&self, claude_features: &[ClaudeFeature]) -> Result<Vec<FeatureGap>>
  }
  ```

### 3. Issue Management (`issue_management.rs`)
- **Responsibility**: Create GitHub issues and maintain ledger
- **Input**: Feature gaps + ledger file + GitHub credentials
- **Output**: List of created issues
- **Stud (Public API)**:
  ```rust
  pub struct IssueManager {
      pub fn new(ledger_path, github_token, repo: impl Into<String>) -> Self
      pub async fn create_issues(&mut self, gaps: &[FeatureGap]) -> Result<Vec<IssueCreated>>
  }
  ```

## Data Files

### Feature Inventory (`.claude/data/feature_inventory.yaml`)
YAML file tracking RustyClawd's implementation status:

```yaml
last_updated: "2025-12-08T00:00:00Z"

features:
  - name: "Bash"
    category: "tools"
    status: complete
    notes: "Fully implemented"

  - name: "MCP Support"
    category: "integrations"
    status: missing
    notes: "Not yet implemented"

  - name: "Inline Diffs"
    category: "ui"
    status: notapplicable
    notes: "CLI doesn't have UI rendering"
```

**Status values:**
- `complete`: Fully implemented with parity
- `partial`: Partially implemented (will trigger gap issue)
- `missing`: Not yet implemented (will trigger gap issue)
- `notapplicable`: Claude Code-specific feature (e.g., UI features in CLI)

### Ledger (`.claude/data/sync_ledger.json`)
JSON file tracking created issues to prevent duplicates:

```json
{
  "issues": {
    "mcp_support": 123,
    "parallel_execution": 124
  },
  "last_sync": "2025-12-08T09:00:00Z"
}
```

## Usage

### Command-Line

```bash
cargo run --package rustyclawd-tools --example claude_code_sync_cli -- \
  --inventory .claude/data/feature_inventory.yaml \
  --ledger .claude/data/sync_ledger.json \
  --token $GITHUB_TOKEN \
  --repo owner/repo
```

### GitHub Action

Runs automatically every Monday at 9 AM UTC (defined in `.github/workflows/claude-code-sync.yml`).

Manual trigger:
```bash
gh workflow run claude-code-sync.yml
```

### Programmatic

```rust
use rustyclawd_tools::claude_code_sync::SyncMonitor;

let mut monitor = SyncMonitor::new(
    ".claude/data/feature_inventory.yaml",
    ".claude/data/sync_ledger.json",
    github_token,
    "owner/repo"
);

let report = monitor.run().await?;
println!("Created {} issues", report.issues_created);
```

## How It Works

1. **Discovery Phase**
   - Fetches Claude Code changelog from Anthropic docs
   - Fetches Claude Code tools documentation
   - Parses markdown to extract feature mentions
   - Deduplicates features by name

2. **Mapping Phase**
   - Loads RustyClawd feature inventory YAML
   - For each Claude Code feature:
     - Searches inventory for match (exact or fuzzy)
     - Determines gap type: Missing, Incomplete, or Drift
     - Skips features marked Complete or NotApplicable

3. **Issue Creation Phase**
   - Loads ledger to check for existing issues
   - For each new gap:
     - Creates GitHub issue with formatted body
     - Records issue number in ledger
     - Labels with `feature-gap` and `claude-code-sync`

4. **Ledger Update**
   - Saves updated ledger with new issue mappings
   - Records sync timestamp

## Testing

```bash
# Run all sync monitor tests
cargo test --package rustyclawd-tools --lib claude_code_sync

# Run specific module tests
cargo test --package rustyclawd-tools --lib claude_code_sync::feature_discovery
cargo test --package rustyclawd-tools --lib claude_code_sync::feature_mapping
cargo test --package rustyclawd-tools --lib claude_code_sync::issue_management
```

Test coverage:
- Feature extraction from text (unit)
- Name normalization and fuzzy matching (unit)
- Empty inventory handling (integration)
- Ledger persistence (integration)
- Issue body formatting (unit)

## Maintenance

### Adding New Features to Inventory

When RustyClawd implements a new Claude Code feature:

1. Add entry to `feature_inventory.yaml`:
   ```yaml
   - name: "NewFeature"
     category: "tools"
     status: complete
     notes: "Implemented in PR #123"
   ```

2. Update `last_updated` timestamp

3. Next sync run won't create gap issue for this feature

### Updating Feature Status

When partial implementation becomes complete:

```yaml
# Before
- name: "PartialFeature"
  status: partial
  notes: "Missing X and Y"

# After
- name: "PartialFeature"
  status: complete
  notes: "Completed in PR #456"
```

### Handling False Positives

If sync monitor creates an issue for a feature that doesn't apply:

1. Close the issue on GitHub
2. Update inventory:
   ```yaml
   - name: "UIOnlyFeature"
     status: notapplicable
     notes: "Claude Code UI feature, not relevant to CLI"
   ```
3. Ledger will still track closed issue, preventing re-creation

## Philosophy Alignment

This implementation follows RustyClawd's development philosophy:

- **Ruthless Simplicity**: Three simple bricks, each with one responsibility
- **Zero-BS Implementation**: All functions work; no stubs or TODOs
- **Modular Design**: Each brick is self-contained and regeneratable
- **Working from Day One**: Real HTTP requests, real GitHub API, real file I/O

## Future Enhancements

Potential improvements (not implemented to maintain simplicity):

- Parse GitHub release notes in addition to changelog
- Support multiple documentation sources
- Configurable gap severity levels
- Email notifications for critical gaps
- Integration with project management tools

## Troubleshooting

### Issue: Features not being discovered

**Cause**: Claude Code documentation structure changed

**Solution**: Update parsing patterns in `feature_discovery.rs::extract_feature_from_line()`

### Issue: Too many false positive gaps

**Cause**: Inventory not up to date

**Solution**: Review and update `feature_inventory.yaml` with current status

### Issue: Duplicate issues being created

**Cause**: Ledger file corrupted or deleted

**Solution**: Regenerate ledger from GitHub issues:
```bash
gh issue list --label "claude-code-sync" --json number,title \
  | jq -r '.[] | "\(.title | split(":")[1] | ltrimstr(" ") | gsub(" "; "_") | ascii_downcase): \(.number)"'
```

## See Also

- [Issue #111](https://github.com/org/repo/issues/111): Original feature request
- `.github/workflows/claude-code-sync.yml`: GitHub Action workflow
- `examples/claude_code_sync_cli.rs`: CLI implementation
