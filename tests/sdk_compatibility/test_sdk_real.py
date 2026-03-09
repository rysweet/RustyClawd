#!/usr/bin/env python3
"""
Real Claude Agent SDK Integration Test

Tests RustyClawd as a drop-in replacement for Claude Code by pointing
the actual Claude Agent SDK at the RustyClawd binary via cli_path.

KNOWN LIMITATION (issue #566):
The SDK uses a bidirectional JSON protocol (--input-format stream-json)
that RustyClawd does not yet implement. The SDK sends initialize/prompt
control messages via stdin and expects JSON responses. This test will
FAIL until #566 is implemented.

Usage:
    ANTHROPIC_API_KEY=sk-... python3 tests/sdk_compatibility/test_sdk_real.py
"""

import asyncio
import os
import sys
import time
from pathlib import Path

# Ensure SDK is available
try:
    from claude_agent_sdk import query, ClaudeAgentOptions
except ImportError:
    print("ERROR: claude-agent-sdk not installed. Run: pip install claude-agent-sdk")
    sys.exit(1)


RUSTY_PATH = Path(__file__).parent.parent.parent / "target" / "release" / "rusty"


async def test_simple_text():
    """Test 1: Simple text response, no tools."""
    print("  [1/4] Simple text response...", end="", flush=True)
    start = time.monotonic()
    messages = []

    async for msg in query(
        prompt="What is 2+2? Answer with just the number.",
        options=ClaudeAgentOptions(
            cli_path=str(RUSTY_PATH),
            allowed_tools=[],
            max_turns=1,
        ),
    ):
        messages.append(msg)

    duration = int((time.monotonic() - start) * 1000)
    # Check we got messages
    has_result = any(hasattr(m, "result") for m in messages)
    has_assistant = any(hasattr(m, "content") for m in messages)

    if has_result or has_assistant:
        result_text = ""
        for m in messages:
            if hasattr(m, "result") and m.result:
                result_text = m.result
        print(f" PASS ({duration}ms, {len(messages)} msgs, result='{result_text[:50]}')")
        return True
    else:
        print(f" FAIL ({duration}ms, {len(messages)} msgs, no result)")
        for m in messages:
            print(f"    msg type: {type(m).__name__}")
        return False


async def test_tool_use_read():
    """Test 2: Read tool use."""
    print("  [2/4] Tool use (Read)...", end="", flush=True)
    start = time.monotonic()
    messages = []

    async for msg in query(
        prompt="Read Cargo.toml and tell me the package name. One word.",
        options=ClaudeAgentOptions(
            cli_path=str(RUSTY_PATH),
            allowed_tools=["Read"],
            max_turns=5,
        ),
    ):
        messages.append(msg)

    duration = int((time.monotonic() - start) * 1000)
    has_result = any(hasattr(m, "result") for m in messages)

    if has_result:
        result_text = ""
        for m in messages:
            if hasattr(m, "result") and m.result:
                result_text = m.result
        print(f" PASS ({duration}ms, {len(messages)} msgs, result='{result_text[:50]}')")
        return True
    else:
        print(f" FAIL ({duration}ms, {len(messages)} msgs)")
        return False


async def test_tool_use_bash():
    """Test 3: Bash tool use."""
    print("  [3/4] Tool use (Bash)...", end="", flush=True)
    start = time.monotonic()
    messages = []

    async for msg in query(
        prompt="Run 'echo hello world' and tell me what it printed. Be brief.",
        options=ClaudeAgentOptions(
            cli_path=str(RUSTY_PATH),
            allowed_tools=["Bash"],
            max_turns=5,
        ),
    ):
        messages.append(msg)

    duration = int((time.monotonic() - start) * 1000)
    has_result = any(hasattr(m, "result") for m in messages)

    if has_result:
        result_text = ""
        for m in messages:
            if hasattr(m, "result") and m.result:
                result_text = m.result
        print(f" PASS ({duration}ms, {len(messages)} msgs, result='{result_text[:50]}')")
        return True
    else:
        print(f" FAIL ({duration}ms, {len(messages)} msgs)")
        return False


async def test_session_id():
    """Test 4: Session ID is returned in init message."""
    print("  [4/4] Session ID...", end="", flush=True)
    start = time.monotonic()
    session_id = None

    async for msg in query(
        prompt="Say ok",
        options=ClaudeAgentOptions(
            cli_path=str(RUSTY_PATH),
            max_turns=1,
        ),
    ):
        if hasattr(msg, "session_id") and msg.session_id:
            session_id = msg.session_id

    duration = int((time.monotonic() - start) * 1000)

    if session_id:
        print(f" PASS ({duration}ms, session_id={session_id})")
        return True
    else:
        print(f" FAIL ({duration}ms, no session_id found)")
        return False


async def main():
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("ERROR: ANTHROPIC_API_KEY not set")
        sys.exit(1)

    if not RUSTY_PATH.exists():
        print(f"ERROR: RustyClawd not found at {RUSTY_PATH}")
        print("Run: cargo build --release")
        sys.exit(1)

    print("=" * 60)
    print("CLAUDE AGENT SDK REAL INTEGRATION TEST")
    print(f"Binary: {RUSTY_PATH}")
    print(f"SDK: claude_agent_sdk v{__import__('claude_agent_sdk').__version__}")
    print("=" * 60)
    print()

    results = []
    results.append(await test_simple_text())
    results.append(await test_tool_use_read())
    results.append(await test_tool_use_bash())
    results.append(await test_session_id())

    passed = sum(results)
    total = len(results)

    print()
    print("=" * 60)
    print(f"RESULT: {passed}/{total} passed")
    print("=" * 60)

    sys.exit(0 if passed == total else 1)


if __name__ == "__main__":
    asyncio.run(main())
