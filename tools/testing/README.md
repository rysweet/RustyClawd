# RustyClawd Continuous Tester 🦀

A Rust-based continuous testing harness that runs RustyClawd in subprocesses to perform comprehensive E2E testing in a loop.

## Features

- **Continuous Loop Testing** - Run tests continuously or for N iterations
- **Subprocess Isolation** - Each test runs in a clean subprocess with depth limiting
- **Detailed Logging** - Captures all output to timestamped log files
- **Visual Feedback** - Colored terminal output with progress indicators
- **Summary Reports** - Comprehensive statistics and success rates
- **Configurable** - Control iterations, delays, scenarios, and output location

## Building

Built as part of the RustyClawd workspace:

```bash
cargo build --release -p rustyclawd-continuous-tester
```

Binary location: `target/release/continuous_tester`

## Usage

### Basic Usage

```bash
# Run 5 iterations with 60 second delays
./target/release/continuous_tester --max-iterations 5 --delay 60

# Run continuously (Ctrl+C to stop)
./target/release/continuous_tester

# Run with custom binary and scenario
./target/release/continuous_tester \
  --binary ./target/debug/rusty \
  --scenario "Test session management and memory" \
  --max-iterations 3
```

### Options

- `--max-iterations N` - Run N iterations then stop (default: unlimited)
- `--delay SECS` - Wait SECS seconds between runs (default: 30)
- `--binary PATH` - Path to rusty binary (default: 'rusty')
- `--scenario NAME` - Test scenario description (default: 'test-all-features')
- `--output DIR` - Output directory for logs (default: `/tmp/rustyclawd-continuous-tests`)
- `--help` - Show help message

### Example Scenarios

```bash
# Test recent features
./target/release/continuous_tester \
  --scenario "Run all E2E tests from tests/e2e/scenarios/" \
  --max-iterations 10

# Test memory system
./target/release/continuous_tester \
  --scenario "Test memory save, load, and search operations" \
  --delay 45

# Test task management
./target/release/continuous_tester \
  --scenario "Create, update, and complete tasks" \
  --max-iterations 20 \
  --delay 15

# Test agent teams
./target/release/continuous_tester \
  --scenario "Run multi-agent coordination tests"

# Stress testing
./target/release/continuous_tester \
  --scenario "Run high-load stress tests with concurrent operations" \
  --delay 120
```

## How It Works

1. **Initialize** - Creates output directory and validates configuration
2. **Loop** - For each iteration:
   - Launches `rusty` in CLI mode with `--max-depth 1` (prevents infinite recursion)
   - Sends the test scenario prompt via stdin
   - Captures stdout/stderr to timestamped log file
   - Monitors for completion or timeout (10 minutes default)
   - Records success/failure and duration
3. **Report** - Prints comprehensive summary with statistics

## Output

Each test run produces:
- **Console output** - Real-time colored progress
- **Log file** - Complete output saved to `/tmp/rustyclawd-continuous-tests/run_NNNN_TIMESTAMP.log`
- **Summary stats** - Total runs, passed, failed, success rate, duration

Example output:
```
🦀 RustyClawd Continuous Tester
================================
Binary: ./target/release/rusty
Scenario: Test all features
Output: /tmp/rustyclawd-continuous-tests
Max iterations: 5
Delay: 30s

▶ Iteration #1
  → Launching rusty subprocess...
  ✅ Test PASSED in 45.23s
  📄 Log: /tmp/rustyclawd-continuous-tests/run_0001_20260211_095152.log

⏳ Waiting 30s before next run...

▶ Iteration #2
  → Launching rusty subprocess...
  ❌ Test FAILED in 12.45s
  📄 Log: /tmp/rustyclawd-continuous-tests/run_0002_20260211_095312.log
  ⚠ Error: Test failed

...

═══════════════════════════════════════
📊 CONTINUOUS TESTING SUMMARY
═══════════════════════════════════════

⏱ Duration: 5m 23s
🔢 Total runs: 5
✅ Passed: 4
❌ Failed: 1
📈 Success rate: 80.0%
```

## Use Cases

### Development Testing
Run continuously while developing to catch regressions:
```bash
./target/release/continuous_tester --delay 60
```

### CI/CD Integration
Run fixed iterations in CI pipeline:
```bash
./target/release/continuous_tester --max-iterations 3 --delay 0
```

### Nightly Testing
Run comprehensive tests overnight:
```bash
./target/release/continuous_tester \
  --scenario "Run full E2E test suite with all scenarios" \
  --max-iterations 50 \
  --delay 300
```

### Stress Testing
Test stability under repeated load:
```bash
./target/release/continuous_tester \
  --scenario "High-load stress test" \
  --max-iterations 100 \
  --delay 10
```

## Safety Features

- **Depth Limiting** - `--max-depth 1` prevents infinite subprocess spawning
- **Timeout** - 10 minute timeout per test prevents hangs
- **Process Cleanup** - Kills subprocess on timeout
- **Error Handling** - Gracefully handles spawn failures and crashes

## Architecture

The continuous tester is:
- Written in **Rust** for cross-platform compatibility
- Uses `std::process` for subprocess management
- Implements timeout with non-blocking `try_wait()`
- Captures output with `BufReader` line-by-line
- Uses `colored` crate for terminal output
- Saves logs with timestamp-based filenames

## Limitations

- Cannot relaunch itself (by design - prevents recursion)
- Single subprocess at a time (no parallelism)
- Limited to CLI mode testing (TUI requires terminal interaction)
- 10 minute timeout per test (configurable in code)

## Contributing

To modify the continuous tester:

1. Edit `tools/testing/continuous_tester.rs`
2. Rebuild: `cargo build --release -p rustyclawd-continuous-tester`
3. Test your changes
4. Submit PR with DEFAULT_WORKFLOW.md

## License

Same as RustyClawd: MIT OR Apache-2.0
