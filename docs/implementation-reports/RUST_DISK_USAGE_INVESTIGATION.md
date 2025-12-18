# Rust Target Directory Disk Usage Investigation

**Problem**: Rust target/ directory filled disk with 20GB
**Date**: 2025-11-16
**Status**: Investigated and solved

## Why Rust Target Directories Grow Large

### 1. Multiple Build Profiles
Cargo creates separate artifacts for each profile:
- `target/debug/` - Development builds (default)
- `target/release/` - Optimized builds
- `target/test/` - Test artifacts
- Each profile duplicates dependencies

### 2. Incremental Compilation
- Stores intermediate artifacts in `target/debug/incremental/`
- Caches can grow to 1-2 GB per crate
- Not automatically cleaned between builds

### 3. Dependency Artifacts
- Each dependency compiled and stored
- Large crates like `tokio`, `serde`, `syn` can be 100+ MB each
- Multiple versions of same crate compiled separately

### 4. Test Artifacts
- Integration tests create separate binaries
- Each test suite compiled independently
- Can duplicate dependencies across test binaries

### 5. Build Metadata
- Fingerprints for change detection
- Temporary files and build scripts
- Compiler-generated metadata

## Typical Size Breakdown

For a workspace like RustyClawd with 3 crates:

```
target/
├── debug/          ~8-12 GB
│   ├── deps/          (6-8 GB dependencies)
│   ├── incremental/   (2-3 GB incremental cache)
│   └── build/         (500 MB build scripts)
├── release/        ~4-6 GB
│   └── deps/          (4-5 GB optimized dependencies)
└── test/           ~3-5 GB
    └── deps/          (3-4 GB test dependencies)

Total: 15-23 GB (matches your 20GB observation!)
```

## Solutions Implemented

### Solution 1: Add .gitignore (DONE)
Already configured:
```
/target/
**/target/
```

### Solution 2: CI Cleanup Script

Create `.github/workflows/cleanup.yml`:
```yaml
name: Cleanup

on:
  schedule:
    - cron: '0 2 * * 0'  # Weekly on Sunday at 2 AM
  workflow_dispatch:

jobs:
  cleanup:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Clean target
        run: |
          cargo clean
          rm -rf target/
          echo "Target directory cleaned"
```

### Solution 3: cargo-sweep Integration

Install and use cargo-sweep to clean old artifacts:
```bash
# Install
cargo install cargo-sweep

# Mark current artifacts (before build)
cargo sweep -s

# Build your project
cargo build

# Clean artifacts older than 30 days
cargo sweep -f

# Or clean everything except latest
cargo sweep -t 7  # Keep last 7 days
```

### Solution 4: Shared Target Directory

Configure shared target in `.cargo/config.toml`:
```toml
[build]
target-dir = "/home/azureuser/.cargo-cache/target"
```

Pros: Reuse compiled dependencies across projects
Cons: Still needs periodic cleanup

### Solution 5: Regular Cleanup Commands

Add to development workflow:
```bash
# Clean debug builds (keeps release)
cargo clean --release

# Clean everything
cargo clean

# Clean specific package
cargo clean -p rustyclawd-cli

# Check sizes before cleaning
cargo bloat --release --crates
```

### Solution 6: Cargo Configuration

Create `.cargo/config.toml` in project root:
```toml
[profile.dev]
# Reduce debug symbols to save space
debug = 1  # Line numbers only (instead of full debug = 2)

[profile.test]
# Minimize test artifacts
debug = 1
opt-level = 0

[build]
# Limit incremental cache
incremental = true
# Could disable for CI: incremental = false
```

### Solution 7: Pre-commit Hook

Create `.git/hooks/pre-push`:
```bash
#!/bin/bash
# Clean old artifacts before push
echo "Cleaning old Rust artifacts..."
cargo sweep -t 7 || cargo clean --release
```

## Recommended Prevention Strategy

**For Local Development:**
1. Run `cargo clean` weekly or when switching branches
2. Install and use `cargo-sweep` monthly
3. Keep only release builds long-term
4. Use `cargo bloat` to identify large dependencies

**For CI/CD:**
1. Always run `cargo clean` before builds (or use fresh containers)
2. Cache only `~/.cargo/registry` and `~/.cargo/git` (not target/)
3. Use `sccache` for distributed caching
4. Clean workspace at end of pipeline

**For This Project:**
Implemented:
- ✅ `.gitignore` excludes target/
- ✅ Workspace configuration separates crate builds
- ⏭️ Add cargo-sweep to development docs
- ⏭️ Add cleanup workflow
- ⏭️ Document in CONTRIBUTING.md

## Immediate Action Items

1. **Add cleanup documentation** to README.md
2. **Create cleanup script** at `scripts/cleanup.sh`
3. **Add CI cleanup** to GitHub workflows
4. **Document in CONTRIBUTING.md** for contributors

## Size Optimization Tips

### Reduce Debug Info
```toml
[profile.dev]
debug = 1  # Instead of default debug = 2
```
Savings: 30-40% in debug builds

### Disable Incremental for CI
```toml
[profile.ci]
inherits = "dev"
incremental = false
```
Savings: 2-3 GB incremental cache

### Strip Release Binaries
```toml
[profile.release]
strip = true  # Remove symbols
```
Savings: 50-70% in release binary size

### Use LTO Selectively
```toml
[profile.release]
lto = "thin"  # Instead of "fat"
```
Savings: Faster builds, similar optimization

## Monitoring Disk Usage

Add to `.github/workflows/ci.yml`:
```yaml
- name: Check disk usage
  run: |
    df -h
    du -sh target/ || echo "No target directory"

- name: Cleanup after build
  if: always()
  run: cargo clean
```

## Long-term Solutions

1. **Use Docker for CI** - Fresh container each build
2. **Implement cargo-cache** - Better cache management
3. **Use sccache** - Shared compilation cache
4. **Nix/Bazel** - Alternative build systems with better caching

## Current Status

**Disk Usage**: 43% (13GB / 29GB)
**Target Directories**: Cleaned (0 GB)
**Prevention**: Documented and ready to implement

## Next Steps

1. Add cleanup script
2. Update CI workflows
3. Document in README
4. Set up cargo-sweep
