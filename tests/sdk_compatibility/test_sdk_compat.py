#!/usr/bin/env python3
"""
Claude Agent SDK Compatibility Test Suite

Runs the same prompts through both Claude Code (official) and RustyClawd,
comparing the SDK message format, types, and behavior side-by-side.

Usage:
    python3 tests/sdk_compatibility/test_sdk_compat.py [--rusty-only] [--claude-only] [--verbose]

Requires:
    - claude-agent-sdk (pip install claude-agent-sdk)
    - claude binary in PATH (official Claude Code)
    - target/release/rusty binary (RustyClawd)
    - ANTHROPIC_API_KEY environment variable
"""

import json
import os
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class MessageRecord:
    """A single message from the SDK stream."""
    msg_type: str  # system, assistant, result, user
    subtype: Optional[str] = None
    has_content: bool = False
    has_session_id: bool = False
    has_parent_tool_use_id: bool = False
    content_types: list = field(default_factory=list)
    model: Optional[str] = None
    num_turns: Optional[int] = None
    is_error: bool = False
    raw: dict = field(default_factory=dict)


@dataclass
class TestResult:
    """Result of running a single test case."""
    test_name: str
    binary: str  # "claude" or "rustyclawd"
    passed: bool
    messages: list  # list of MessageRecord
    duration_ms: int = 0
    error: Optional[str] = None
    raw_output: str = ""


def run_cli_stream_json(binary_path: str, prompt: str, timeout: int = 60) -> tuple[list[dict], str, int]:
    """Run a binary with --print --output-format stream-json and capture messages."""
    start = time.monotonic()
    try:
        # Clean env: remove ALL nested session guards so claude can run from within claude
        guard_vars = ("CLAUDECODE", "CLAUDE_CODE_SESSION", "CLAUDE_CODE_ENTRYPOINT",
                       "CLAUDE_PLUGIN_ROOT", "CLAUDE_CODE_SIMPLE")
        clean_env = {k: v for k, v in os.environ.items() if k not in guard_vars}
        clean_env["CLAUDE_CODE_MAX_OUTPUT_TOKENS"] = "1024"

        result = subprocess.run(
            [binary_path, "--print", "--output-format", "stream-json", prompt],
            capture_output=True,
            text=True,
            timeout=timeout,
            env=clean_env,
        )
        duration = int((time.monotonic() - start) * 1000)
        raw = result.stdout + result.stderr

        # Parse newline-delimited JSON
        messages = []
        for line in result.stdout.strip().split("\n"):
            line = line.strip()
            if not line:
                continue
            try:
                messages.append(json.loads(line))
            except json.JSONDecodeError:
                pass  # Skip non-JSON lines (progress indicators, etc.)

        return messages, raw, duration
    except subprocess.TimeoutExpired:
        duration = int((time.monotonic() - start) * 1000)
        return [], f"TIMEOUT after {timeout}s", duration
    except Exception as e:
        duration = int((time.monotonic() - start) * 1000)
        return [], str(e), duration


def parse_messages(raw_messages: list[dict]) -> list[MessageRecord]:
    """Parse raw JSON messages into MessageRecord objects."""
    records = []
    for msg in raw_messages:
        record = MessageRecord(
            msg_type=msg.get("type", "unknown"),
            subtype=msg.get("subtype"),
            has_content="content" in msg,
            has_session_id="session_id" in msg,
            has_parent_tool_use_id="parent_tool_use_id" in msg,
            model=msg.get("model"),
            num_turns=msg.get("num_turns"),
            is_error=msg.get("is_error", False),
            raw=msg,
        )
        if isinstance(msg.get("content"), list):
            record.content_types = [
                block.get("type", "unknown") if isinstance(block, dict) else "text"
                for block in msg["content"]
            ]
        records.append(record)
    return records


def compare_message_formats(claude_records: list[MessageRecord],
                           rusty_records: list[MessageRecord]) -> list[str]:
    """Compare message formats between Claude Code and RustyClawd."""
    diffs = []

    # Check message type sequence
    claude_types = [(r.msg_type, r.subtype) for r in claude_records]
    rusty_types = [(r.msg_type, r.subtype) for r in rusty_records]

    if not rusty_records:
        diffs.append("RustyClawd produced no messages")
        return diffs

    if not claude_records:
        diffs.append("Claude Code produced no messages")
        return diffs

    # Check for init message
    claude_has_init = any(r.msg_type == "system" and r.subtype == "init" for r in claude_records)
    rusty_has_init = any(r.msg_type == "system" and r.subtype == "init" for r in rusty_records)
    if claude_has_init and not rusty_has_init:
        diffs.append("MISSING: RustyClawd does not emit init message")
    elif rusty_has_init and not claude_has_init:
        diffs.append("EXTRA: RustyClawd emits init message but Claude Code does not")

    # Check for result message
    claude_has_result = any(r.msg_type == "result" for r in claude_records)
    rusty_has_result = any(r.msg_type == "result" for r in rusty_records)
    if claude_has_result and not rusty_has_result:
        diffs.append("MISSING: RustyClawd does not emit result message")

    # Check for assistant messages
    claude_assistant = [r for r in claude_records if r.msg_type == "assistant"]
    rusty_assistant = [r for r in rusty_records if r.msg_type == "assistant"]
    if claude_assistant and not rusty_assistant:
        diffs.append("MISSING: RustyClawd does not emit assistant messages")

    # Check session_id presence
    claude_session_ids = [r.has_session_id for r in claude_records]
    rusty_session_ids = [r.has_session_id for r in rusty_records]
    if any(claude_session_ids) and not any(rusty_session_ids):
        diffs.append("MISSING: RustyClawd messages lack session_id")

    # Check content block types match
    if claude_assistant and rusty_assistant:
        claude_ctypes = set()
        rusty_ctypes = set()
        for r in claude_assistant:
            claude_ctypes.update(r.content_types)
        for r in rusty_assistant:
            rusty_ctypes.update(r.content_types)
        missing_types = claude_ctypes - rusty_ctypes
        if missing_types:
            diffs.append(f"MISSING content block types: {missing_types}")

    return diffs


# ============================================================================
# TEST CASES
# ============================================================================

TEST_CASES = [
    {
        "name": "simple_text_response",
        "prompt": "What is 2+2? Answer with just the number.",
        "description": "Basic text-only response, no tool use",
        "checks": ["has_assistant", "has_result", "no_error"],
    },
    {
        "name": "tool_use_read",
        "prompt": "Read the file Cargo.toml and tell me the package name. Be brief.",
        "description": "Single tool use (Read), then text response",
        "checks": ["has_assistant", "has_result", "used_tools"],
    },
    {
        "name": "tool_use_bash",
        "prompt": "Run 'echo hello' and tell me what it printed. Be brief.",
        "description": "Bash tool use, then text response",
        "checks": ["has_assistant", "has_result", "used_tools"],
    },
    {
        "name": "multi_turn",
        "prompt": "List the files in the crates/ directory, then tell me how many there are. Be brief.",
        "description": "Multiple tool calls in sequence",
        "checks": ["has_assistant", "has_result", "multiple_turns"],
    },
]


def run_checks(records: list[MessageRecord], checks: list[str]) -> list[str]:
    """Run validation checks on message records."""
    failures = []
    for check in checks:
        if check == "has_assistant":
            if not any(r.msg_type == "assistant" for r in records):
                failures.append("No assistant message found")
        elif check == "has_result":
            if not any(r.msg_type == "result" for r in records):
                failures.append("No result message found")
        elif check == "no_error":
            if any(r.is_error for r in records):
                failures.append("Error message found")
        elif check == "used_tools":
            # Check for tool_use in content blocks
            has_tool = False
            for r in records:
                if "tool_use" in r.content_types or "ToolUse" in str(r.raw.get("content", [])):
                    has_tool = True
                    break
            # Also check if multiple assistant messages (indicates tool loop)
            assistant_count = sum(1 for r in records if r.msg_type == "assistant")
            if not has_tool and assistant_count < 2:
                failures.append("No tool use detected")
        elif check == "multiple_turns":
            result_msgs = [r for r in records if r.msg_type == "result"]
            if result_msgs and result_msgs[0].num_turns is not None:
                if result_msgs[0].num_turns < 2:
                    failures.append(f"Expected multiple turns, got {result_msgs[0].num_turns}")
    return failures


def print_report(results: list[TestResult], verbose: bool = False):
    """Print the test report."""
    print("\n" + "=" * 70)
    print("CLAUDE AGENT SDK COMPATIBILITY TEST REPORT")
    print("=" * 70)

    # Group by test name
    by_test = {}
    for r in results:
        by_test.setdefault(r.test_name, {})[r.binary] = r

    total_pass = 0
    total_fail = 0

    for test_name, binaries in by_test.items():
        print(f"\n--- {test_name} ---")
        for binary, result in binaries.items():
            status = "PASS" if result.passed else "FAIL"
            icon = "+" if result.passed else "x"
            print(f"  [{icon}] {binary}: {status} ({result.duration_ms}ms, {len(result.messages)} messages)")
            if result.passed:
                total_pass += 1
            else:
                total_fail += 1
            if result.error:
                print(f"      Error: {result.error}")
            if verbose:
                for msg in result.messages:
                    print(f"      {msg.msg_type}/{msg.subtype}: content_types={msg.content_types}")

        # Compare if both exist
        claude_result = binaries.get("claude")
        rusty_result = binaries.get("rustyclawd")
        if claude_result and rusty_result and claude_result.messages and rusty_result.messages:
            diffs = compare_message_formats(claude_result.messages, rusty_result.messages)
            if diffs:
                print(f"  Format diffs:")
                for d in diffs:
                    print(f"    - {d}")
            else:
                print(f"  Format: COMPATIBLE")

    print(f"\n{'=' * 70}")
    print(f"TOTAL: {total_pass} passed, {total_fail} failed out of {total_pass + total_fail}")
    print(f"{'=' * 70}\n")

    return total_fail == 0


def main():
    verbose = "--verbose" in sys.argv or "-v" in sys.argv
    rusty_only = "--rusty-only" in sys.argv
    claude_only = "--claude-only" in sys.argv

    # Check prerequisites
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("ERROR: ANTHROPIC_API_KEY not set")
        print("Note: Claude Code uses OAuth auth, not API key.")
        print("      Use --rusty-only if you only have an API key.")
        sys.exit(1)

    claude_path = "claude"
    rusty_path = str(Path(__file__).parent.parent.parent / "target" / "release" / "rusty")

    if not Path(rusty_path).exists() and not claude_only:
        print(f"ERROR: RustyClawd binary not found at {rusty_path}")
        print("Run: cargo build --release")
        sys.exit(1)

    results = []

    for tc in TEST_CASES:
        print(f"\nRunning: {tc['name']} - {tc['description']}")

        if not rusty_only:
            print(f"  Running claude...", end="", flush=True)
            msgs, raw, duration = run_cli_stream_json(claude_path, tc["prompt"])
            records = parse_messages(msgs)
            failures = run_checks(records, tc["checks"])
            results.append(TestResult(
                test_name=tc["name"],
                binary="claude",
                passed=len(failures) == 0,
                messages=records,
                duration_ms=duration,
                error="; ".join(failures) if failures else None,
                raw_output=raw[:500],
            ))
            print(f" done ({duration}ms, {len(records)} msgs)")

        if not claude_only:
            print(f"  Running rustyclawd...", end="", flush=True)
            msgs, raw, duration = run_cli_stream_json(rusty_path, tc["prompt"])
            records = parse_messages(msgs)
            failures = run_checks(records, tc["checks"])
            results.append(TestResult(
                test_name=tc["name"],
                binary="rustyclawd",
                passed=len(failures) == 0,
                messages=records,
                duration_ms=duration,
                error="; ".join(failures) if failures else None,
                raw_output=raw[:500],
            ))
            print(f" done ({duration}ms, {len(records)} msgs)")

    all_passed = print_report(results, verbose)
    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    main()
