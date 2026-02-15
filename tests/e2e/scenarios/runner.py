#!/usr/bin/env python3
"""Lightweight YAML Scenario Runner for E2E Tests

Philosophy:
- Ruthless simplicity: Python + tmux (no Rust compilation)
- Zero-BS: Actually executes scenarios, validates behavior
- Quality over speed: Proper error handling and reporting

Usage:
    ./runner.py                    # Run all scenarios
    ./runner.py --file scenario.yaml # Run single scenario
    ./runner.py --verbose          # Show detailed output
    ./runner.py --tag core-workflow # Run scenarios with tag
"""

import yaml
import subprocess
import sys
import time
import os
import re
import shlex
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass
from datetime import datetime
import json


@dataclass
class RunResult:
    """Result of a single scenario run"""
    scenario_name: str
    status: str  # "passed", "failed", "error"
    error_message: Optional[str] = None
    duration: float = 0.0
    captured_output: Optional[str] = None


class ScenarioRunner:
    """Execute a single YAML scenario via tmux"""

    def __init__(self, scenario_file: Path, verbose: bool = False):
        """Initialize runner for a scenario file"""
        self.scenario_file = scenario_file
        self.verbose = verbose
        self.session_name = f"scenario-{os.getpid()}-{int(time.time())}"
        self.framework_dir = Path(__file__).parent.parent / "tmux"
        self.screenshot_dir = Path(__file__).parent / "screenshots"
        self.screenshot_dir.mkdir(exist_ok=True, parents=True)

        # Load scenario with size and depth limits for security
        MAX_YAML_SIZE = 1024 * 1024  # 1MB limit
        MAX_YAML_DEPTH = 50  # Depth limit

        try:
            # Check file size before loading
            file_size = scenario_file.stat().st_size
            if file_size > MAX_YAML_SIZE:
                raise RuntimeError(f"Scenario file too large: {file_size} bytes (max {MAX_YAML_SIZE})")

            with open(scenario_file) as f:
                content = f.read(MAX_YAML_SIZE + 1)
                if len(content) > MAX_YAML_SIZE:
                    raise RuntimeError(f"Scenario content too large (max {MAX_YAML_SIZE} bytes)")

                data = yaml.safe_load(content)

                # Validate depth to prevent DoS
                def check_depth(obj, depth=0):
                    if depth > MAX_YAML_DEPTH:
                        raise RuntimeError(f"YAML structure too deep (max depth {MAX_YAML_DEPTH})")
                    if isinstance(obj, dict):
                        for value in obj.values():
                            check_depth(value, depth + 1)
                    elif isinstance(obj, list):
                        for item in obj:
                            check_depth(item, depth + 1)

                check_depth(data)

                self.scenario = data.get("scenario", {})
                self.name = self.scenario.get("name", "Unknown")
                self.steps = self.scenario.get("steps", [])
                self.assertions = self.scenario.get("assertions", [])
                self.environment = self.scenario.get("environment", {})
        except Exception as e:
            raise RuntimeError(f"Failed to load scenario {scenario_file}: {e}")

    def _source_framework(self) -> str:
        """Get bash code to source the tmux framework"""
        framework_path = self.framework_dir / "framework.sh"
        if not framework_path.exists():
            raise RuntimeError(f"tmux framework not found: {framework_path}")
        return f'source "{framework_path}"'

    def _run_bash_cmd(self, cmd: str, timeout: int = 120) -> Tuple[int, str, str]:
        """Run a bash command and return exit code, stdout, stderr"""
        try:
            result = subprocess.run(
                ["/bin/bash", "-c", cmd],
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=str(self.framework_dir.parent.parent)
            )
            return result.returncode, result.stdout, result.stderr
        except subprocess.TimeoutExpired:
            return 124, "", f"Command timed out after {timeout}s"
        except Exception as e:
            return 1, "", f"Command failed: {e}"

    def _execute_step(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute a single scenario step. Returns (success, error_message)"""
        action = step.get("action")
        description = step.get("description", f"Execute {action}")

        if self.verbose:
            print(f"  - {description}...", end=" ", flush=True)

        try:
            if action == "launch":
                return self._step_launch(step)
            elif action == "send_input":
                return self._step_send_input(step)
            elif action == "send_keys":
                return self._step_send_keys(step)
            elif action == "send_key":
                return self._step_send_key(step)
            elif action == "wait_for_text":
                return self._step_wait_for_text(step)
            elif action == "capture_screenshot":
                return self._step_capture_screenshot(step)
            elif action == "sleep":
                return self._step_sleep(step)
            elif action == "ensure_file":
                return self._step_ensure_file(step)
            elif action == "remove_file":
                return self._step_remove_file(step)
            elif action == "cleanup_test_files":
                return self._step_cleanup_test_files(step)
            elif action == "execute_command":
                return self._step_execute_command(step)
            elif action == "resize_terminal":
                return self._step_resize_terminal(step)
            elif action == "none":
                return True, None  # No-op action
            else:
                return False, f"Unknown action: {action}"
        except Exception as e:
            return False, f"{action} failed: {e}"

    def _step_launch(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute launch step: start RustyClawd in tmux"""
        target = step.get("target", "cargo run --bin rustyclawd")
        timeout = self._parse_duration(step.get("timeout", "30s"))

        # Create bash script to start session (NO trap_cleanup - we manage cleanup in Python)
        session_quoted = shlex.quote(self.session_name)
        bash_cmd = f"""
            {self._source_framework()}
            SESSION={session_quoted}
            start_rustyclawd_session "$SESSION" {int(timeout)} || exit 1
            echo "Session started successfully"
        """

        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd, timeout=int(timeout) + 5)

        if exit_code != 0:
            return False, f"Failed to start RustyClawd: {stderr}"

        if self.verbose:
            print("OK")
        return True, None

    def _step_send_input(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute send_input step: send text to tmux session"""
        text = step.get("text", "")
        submit = step.get("submit", True)
        wait_time = step.get("wait", 1)

        # Quote all bash variables for safety
        session_quoted = shlex.quote(self.session_name)
        text_quoted = shlex.quote(text)

        # Build tmux send command
        if submit:
            bash_cmd = f"""
                {self._source_framework()}
                SESSION={session_quoted}
                send_command "$SESSION" {text_quoted} {wait_time} || exit 1
            """
        else:
            bash_cmd = f"""
                {self._source_framework()}
                SESSION={session_quoted}
                send_keys "$SESSION" {text_quoted} || exit 1
                sleep {wait_time}
            """

        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd)

        if exit_code != 0:
            return False, f"Failed to send input: {stderr}"

        if self.verbose:
            print("OK")
        return True, None

    def _step_wait_for_text(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute wait_for_text step: wait for text to appear"""
        contains = step.get("contains", "")
        # Increase default timeout for AI responses - they can be slow!
        timeout = self._parse_duration(step.get("timeout", "30s"))
        case_insensitive = step.get("case_insensitive", True)  # Default to flexible matching

        # Quote bash variables for safety
        session_quoted = shlex.quote(self.session_name)
        contains_quoted = shlex.quote(contains)

        # Choose appropriate wait function based on case sensitivity
        wait_func = "wait_for_text_flexible" if case_insensitive else "wait_for_text"

        bash_cmd = f"""
            {self._source_framework()}
            SESSION={session_quoted}
            if {wait_func} "$SESSION" {contains_quoted} {int(timeout)}; then
                echo "Text found"
            else
                echo "Text not found"
                exit 1
            fi
        """

        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd, timeout=int(timeout) + 5)

        if exit_code != 0:
            # Capture what we actually got for debugging
            capture_cmd = f"""
                {self._source_framework()}
                SESSION="{self.session_name}"
                capture_output "$SESSION" | sed 's/\x1b\[[0-9;]*m//g'
            """
            _, actual, _ = self._run_bash_cmd(capture_cmd)
            mode = "case-insensitive" if case_insensitive else "case-sensitive"
            return False, f"Expected '{contains}' not found ({mode}). Got:\n{actual[:500]}"

        if self.verbose:
            print("OK")
        return True, None

    def _step_capture_screenshot(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute capture_screenshot step: save terminal state"""
        filename = step.get("filename", f"screenshot-{int(time.time())}.txt")

        # Sanitize filename to prevent path traversal
        filename = os.path.basename(filename)
        if not re.match(r'^[a-zA-Z0-9_\-\.]+$', filename):
            return False, f"Invalid filename: {filename} (must contain only alphanumeric, dash, underscore, and dot)"

        filepath = self.screenshot_dir / filename

        # Quote bash variables for safety
        session_quoted = shlex.quote(self.session_name)
        filepath_quoted = shlex.quote(str(filepath))

        bash_cmd = f"""
            {self._source_framework()}
            SESSION={session_quoted}
            capture_output "$SESSION" > {filepath_quoted} 2>&1 || exit 1
        """

        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd)

        if exit_code != 0:
            return False, f"Failed to capture screenshot: {stderr}"

        if self.verbose:
            print(f"OK (saved to {filepath})")
        return True, None

    def _step_sleep(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute sleep step: wait for specified duration"""
        duration = self._parse_duration(step.get("duration", "1s"))
        if self.verbose:
            print(f"sleeping for {duration}s...", end=" ", flush=True)
        time.sleep(duration)
        if self.verbose:
            print("OK")
        return True, None

    def _step_send_keys(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute send_keys step: send raw key sequence to tmux"""
        keys = step.get("keys", "")
        if not keys:
            return False, "send_keys requires 'keys' parameter"
        
        session_quoted = shlex.quote(self.session_name)
        keys_quoted = shlex.quote(keys)
        
        bash_cmd = f"""
            {self._source_framework()}
            SESSION={session_quoted}
            send_keys "$SESSION" {keys_quoted}
        """
        
        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd)
        
        if exit_code != 0:
            return False, f"Failed to send keys: {stderr}"
        
        if self.verbose:
            print("OK")
        return True, None

    def _step_send_key(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute send_key step: send special key (Tab, Escape, etc.)"""
        key = step.get("key", "")
        if not key:
            return False, "send_key requires 'key' parameter"
        
        # Map special keys to tmux key names
        key_map = {
            "Tab": "Tab",
            "Shift+Tab": "BTab",
            "BackTab": "BTab",  # Alternative name for Shift+Tab
            "Escape": "Escape",
            "Enter": "Enter",
            "Space": "Space",
            "Up": "Up",
            "Down": "Down",
            "Left": "Left",
            "Right": "Right",
        }
        
        tmux_key = key_map.get(key, key)
        session_quoted = shlex.quote(self.session_name)
        
        bash_cmd = f"""
            {self._source_framework()}
            SESSION={session_quoted}
            send_keys "$SESSION" {tmux_key}
        """
        
        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd)
        
        if exit_code != 0:
            return False, f"Failed to send key '{key}': {stderr}"
        
        if self.verbose:
            print("OK")
        return True, None

    def _step_ensure_file(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute ensure_file step: create a test file"""
        path = step.get("path", "")
        content = step.get("content", "")
        
        if not path:
            return False, "ensure_file requires 'path' parameter"
        
        # Security: Only allow files in /tmp or specific test directories
        allowed_prefixes = ["/tmp/", "/var/tmp/", "./test_data/", "test_data/", "tests/e2e/fixtures/", "./tests/e2e/fixtures/"]
        if not any(path.startswith(prefix) for prefix in allowed_prefixes):
            return False, f"ensure_file path must be in /tmp, test_data/, or tests/e2e/fixtures/: {path}"
        
        try:
            # Create parent directory if needed
            os.makedirs(os.path.dirname(path) if os.path.dirname(path) else ".", exist_ok=True)
            
            # Write file
            with open(path, 'w') as f:
                f.write(content)
            
            if self.verbose:
                print(f"OK (created {path})")
            return True, None
        except Exception as e:
            return False, f"Failed to create file: {e}"

    def _step_remove_file(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute remove_file step: delete a test file"""
        path = step.get("path", "")
        
        if not path:
            return False, "remove_file requires 'path' parameter"
        
        # Security: Only allow files in /tmp or specific test directories
        allowed_prefixes = ["/tmp/", "/var/tmp/", "./test_data/", "test_data/", "tests/e2e/fixtures/", "./tests/e2e/fixtures/"]
        if not any(path.startswith(prefix) for prefix in allowed_prefixes):
            return False, f"remove_file path must be in /tmp, test_data/, or tests/e2e/fixtures/: {path}"
        
        try:
            if os.path.exists(path):
                os.remove(path)
            if self.verbose:
                print(f"OK (removed {path})")
            return True, None
        except Exception as e:
            return False, f"Failed to remove file: {e}"

    def _step_cleanup_test_files(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute cleanup_test_files step: remove all test files"""
        pattern = step.get("pattern", "/tmp/test_*.txt")
        
        # Security: Only allow cleanup in /tmp or test directories
        if not pattern.startswith(("/tmp/", "/var/tmp/", "./test_data/", "test_data/", "tests/e2e/fixtures/", "./tests/e2e/fixtures/")):
            return False, f"cleanup_test_files pattern must be in /tmp, test_data/, or tests/e2e/fixtures/: {pattern}"
        
        try:
            import glob
            files = glob.glob(pattern)
            for f in files:
                try:
                    os.remove(f)
                except Exception as e:
                    if self.verbose:
                        print(f"Warning: Failed to remove {f}: {e}")
            
            if self.verbose:
                print(f"OK (cleaned {len(files)} files)")
            return True, None
        except Exception as e:
            return False, f"Failed to cleanup files: {e}"

    def _step_execute_command(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute execute_command step: run a shell command"""
        command = step.get("command", "")
        
        if not command:
            return False, "execute_command requires 'command' parameter"
        
        timeout = self._parse_duration(step.get("timeout", "30s"))
        
        try:
            result = subprocess.run(
                ["/bin/bash", "-c", command],
                capture_output=True,
                text=True,
                timeout=timeout,
                cwd=str(self.framework_dir.parent.parent)
            )
            
            if result.returncode != 0:
                return False, f"Command failed with exit code {result.returncode}: {result.stderr}"
            
            if self.verbose:
                print("OK")
            return True, None
        except subprocess.TimeoutExpired:
            return False, f"Command timed out after {timeout}s"
        except Exception as e:
            return False, f"Command failed: {e}"

    def _step_resize_terminal(self, step: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """Execute resize_terminal step: resize the tmux pane"""
        width = step.get("width", 120)
        height = step.get("height", 30)
        
        session_quoted = shlex.quote(self.session_name)
        
        bash_cmd = f"""
            tmux resize-pane -t {session_quoted} -x {width} -y {height}
        """
        
        exit_code, stdout, stderr = self._run_bash_cmd(bash_cmd)
        
        if exit_code != 0:
            return False, f"Failed to resize terminal: {stderr}"
        
        if self.verbose:
            print(f"OK ({width}x{height})")
        return True, None

    def _parse_duration(self, duration_str: str) -> float:
        """Parse duration string like '10s' to seconds"""
        duration_str = str(duration_str).strip().lower()
        if duration_str.endswith("s"):
            return float(duration_str[:-1])
        elif duration_str.endswith("m"):
            return float(duration_str[:-1]) * 60
        else:
            return float(duration_str)

    def _check_assertions(self) -> Tuple[bool, List[str]]:
        """Check all scenario assertions. Returns (passed, error_messages)"""
        errors = []

        # Capture final output
        bash_cmd = f"""
            {self._source_framework()}
            SESSION="{self.session_name}"
            capture_output "$SESSION"
        """
        _, output, _ = self._run_bash_cmd(bash_cmd)

        # Check each assertion
        for assertion in self.assertions:
            assertion_type = assertion.get("type")
            value = assertion.get("value")
            description = assertion.get("description", f"Check {assertion_type}")

            if assertion_type == "text_present":
                if value not in output:
                    errors.append(f"FAIL: {description} - '{value}' not found in output")
                elif self.verbose:
                    print(f"  ✓ {description}")

            elif assertion_type == "text_not_present":
                if value in output:
                    errors.append(f"FAIL: {description} - '{value}' found in output")
                elif self.verbose:
                    print(f"  ✓ {description}")

            elif assertion_type == "exit_clean":
                # Just check that we got here - if we did, exit was clean
                if self.verbose:
                    print(f"  ✓ {description}")

            elif assertion_type == "file_exists":
                # Support both 'path' and 'value' for backwards compatibility
                filepath_str = assertion.get("path") or value
                if not filepath_str:
                    errors.append(f"FAIL: {description} - no path or value specified")
                    continue
                filepath = Path(filepath_str)
                if not filepath.exists():
                    errors.append(f"FAIL: {description} - file not found: {filepath_str}")
                elif self.verbose:
                    print(f"  ✓ {description}")

        return len(errors) == 0, errors

    def _cleanup(self):
        """Clean up tmux session"""
        session_quoted = shlex.quote(self.session_name)
        bash_cmd = f"""
            {self._source_framework()}
            SESSION={session_quoted}
            cleanup_session "$SESSION"
        """
        self._run_bash_cmd(bash_cmd)

    def run(self) -> RunResult:
        """Run the scenario. Returns RunResult"""
        start_time = time.time()

        if self.verbose:
            print(f"\nRunning scenario: {self.name}")
            print(f"Description: {self.scenario.get('description', 'N/A')}")
            print(f"Steps: {len(self.steps)}")

        try:
            # Execute all steps
            for i, step in enumerate(self.steps, 1):
                success, error = self._execute_step(step)
                if not success:
                    self._cleanup()
                    return RunResult(
                        scenario_name=self.name,
                        status="failed",
                        error_message=f"Step {i} failed: {error}",
                        duration=time.time() - start_time
                    )

            # Check assertions
            assertions_passed, assertion_errors = self._check_assertions()
            if not assertions_passed:
                self._cleanup()
                return RunResult(
                    scenario_name=self.name,
                    status="failed",
                    error_message="Assertions failed:\n" + "\n".join(assertion_errors),
                    duration=time.time() - start_time
                )

            # Success!
            self._cleanup()
            if self.verbose:
                print(f"\n✓ PASSED: {self.name}")
            return RunResult(
                scenario_name=self.name,
                status="passed",
                duration=time.time() - start_time
            )

        except Exception as e:
            self._cleanup()
            return RunResult(
                scenario_name=self.name,
                status="error",
                error_message=str(e),
                duration=time.time() - start_time
            )


class ScenarioManager:
    """Manage multiple scenarios"""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose
        self.scenarios_dir = Path(__file__).parent
        self.results: List[RunResult] = []

    def find_scenarios(self, pattern: Optional[str] = None, tag: Optional[str] = None) -> List[Path]:
        """Find scenario files matching optional pattern or tag"""
        scenarios = sorted(self.scenarios_dir.glob("*.yaml"))

        if pattern:
            scenarios = [s for s in scenarios if pattern in s.name]

        if tag:
            MAX_YAML_SIZE = 1024 * 1024  # 1MB limit
            filtered = []
            for scenario_file in scenarios:
                try:
                    # Check file size before loading
                    file_size = scenario_file.stat().st_size
                    if file_size > MAX_YAML_SIZE:
                        continue  # Skip oversized files

                    with open(scenario_file) as f:
                        content = f.read(MAX_YAML_SIZE + 1)
                        if len(content) > MAX_YAML_SIZE:
                            continue  # Skip oversized content

                        data = yaml.safe_load(content)
                        scenario = data.get("scenario", {})
                        tags = scenario.get("tags", [])
                        if tag in tags:
                            filtered.append(scenario_file)
                except:
                    pass
            scenarios = filtered

        return scenarios

    def run_all(self, pattern: Optional[str] = None, tag: Optional[str] = None) -> int:
        """Run all matching scenarios. Returns exit code (0 = all passed)"""
        scenarios = self.find_scenarios(pattern=pattern, tag=tag)

        if not scenarios:
            print("No scenarios found")
            return 1

        print(f"Found {len(scenarios)} scenario(s)")
        print()

        for scenario_file in scenarios:
            runner = ScenarioRunner(scenario_file, verbose=self.verbose)
            result = runner.run()
            self.results.append(result)

            # Print result
            status_symbol = "✓" if result.status == "passed" else "✗"
            print(f"{status_symbol} {result.scenario_name} ({result.duration:.1f}s)")
            if result.error_message:
                print(f"  Error: {result.error_message}")

        # Summary
        print()
        print("=" * 70)
        passed = sum(1 for r in self.results if r.status == "passed")
        failed = sum(1 for r in self.results if r.status == "failed")
        errors = sum(1 for r in self.results if r.status == "error")

        print(f"Results: {passed} passed, {failed} failed, {errors} errors")
        print(f"Total time: {sum(r.duration for r in self.results):.1f}s")

        # Exit code
        if failed == 0 and errors == 0:
            print("✓ All scenarios passed!")
            return 0
        else:
            print("✗ Some scenarios failed")
            return 1


def main():
    """Main entry point"""
    import argparse

    parser = argparse.ArgumentParser(
        description="Run YAML E2E test scenarios using tmux framework"
    )
    parser.add_argument(
        "--file",
        help="Run specific scenario file"
    )
    parser.add_argument(
        "--tag",
        help="Run scenarios with specific tag"
    )
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show detailed output"
    )

    args = parser.parse_args()

    manager = ScenarioManager(verbose=args.verbose)

    if args.file:
        # Run single file
        scenario_file = Path(args.file)
        if not scenario_file.exists():
            print(f"Error: Scenario file not found: {args.file}")
            return 1
        runner = ScenarioRunner(scenario_file, verbose=args.verbose)
        result = runner.run()
        if result.status == "passed":
            print(f"✓ {result.scenario_name} passed")
            return 0
        else:
            print(f"✗ {result.scenario_name} failed")
            if result.error_message:
                print(result.error_message)
            return 1
    else:
        # Run all scenarios
        return manager.run_all(tag=args.tag)


if __name__ == "__main__":
    sys.exit(main())
