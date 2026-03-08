---
name: simplify
description: Review changed code for reuse, quality, and efficiency, then fix any issues found
---

# Simplify Skill

## Purpose

Review recently changed files for code reuse opportunities, code quality issues, and efficiency improvements. Spawns three parallel review agents, aggregates their findings, and applies fixes directly.

## When This Skill Activates

- User invokes `/simplify`
- User invokes `/simplify <specific concern>` to focus the review
- User asks to "simplify", "clean up", or "review recent changes"

## Input

- **Optional text argument**: Focus area for the review (e.g., "focus on error handling patterns" or "check for duplicated validation logic")
- If no argument provided, reviews all three dimensions equally

## Process

### Step 1: Identify Changed Files

Determine which files have recently changed. Use git to find them:

```bash
# Files changed in the current branch vs main
git diff --name-only main...HEAD

# If no branch diff, use recent commits
git diff --name-only HEAD~5

# If working tree has uncommitted changes, include those
git diff --name-only
```

Collect the union of changed files. Filter to source code files only (`.rs`, `.py`, `.ts`, `.js`, `.go`, `.toml`, `.yaml`, `.json`, etc.). Exclude generated files, lock files, and binary files.

Read all identified changed files to understand the full context of recent modifications.

### Step 2: Spawn Three Review Agents in Parallel

Launch three independent review agents simultaneously using the Task tool. Each agent receives the list of changed files and their contents.

If the user provided a focus argument, pass it to all three agents as additional context to weight their analysis.

#### Agent 1: Code Reuse Reviewer

Prompt for this agent:

```
You are a code reuse reviewer. Analyze the following changed files for:

1. **Duplicated logic**: Functions or blocks that do the same thing in different places
2. **Missed abstractions**: Repeated patterns that could be extracted into a shared function or module
3. **Copy-paste code**: Near-identical code blocks with minor variations
4. **Reinvented wheels**: Custom implementations of functionality already available in the standard library or existing project utilities
5. **Consolidation opportunities**: Multiple small functions that could be unified into a single parameterized function

For each finding, provide:
- File path and line numbers
- Description of the reuse opportunity
- Concrete suggestion with example code showing the fix
- Estimated impact (high/medium/low)

Focus argument from user (if any): {focus_argument}

Changed files:
{file_contents}
```

#### Agent 2: Code Quality Reviewer

Prompt for this agent:

```
You are a code quality reviewer. Analyze the following changed files for:

1. **Naming issues**: Unclear variable/function/type names that don't convey intent
2. **Error handling gaps**: Missing error handling, swallowed errors, or generic catch-all handlers
3. **Dead code**: Unreachable code, unused variables, commented-out blocks
4. **Complexity**: Functions doing too many things, deep nesting, long parameter lists
5. **Documentation gaps**: Public functions or complex logic missing documentation
6. **Type safety**: Missing type annotations, unsafe type conversions, or unclear data flow
7. **Unsafe patterns**: Unwrap calls without context, index access without bounds checking

For each finding, provide:
- File path and line numbers
- Description of the quality issue
- Concrete fix with example code
- Severity (critical/major/minor)

Focus argument from user (if any): {focus_argument}

Changed files:
{file_contents}
```

#### Agent 3: Efficiency Reviewer

Prompt for this agent:

```
You are an efficiency reviewer. Analyze the following changed files for:

1. **Unnecessary allocations**: Creating strings, vectors, or objects that could be avoided
2. **Redundant operations**: Repeated computations, unnecessary clones, or duplicate I/O
3. **Algorithm issues**: O(n^2) patterns where O(n) or O(n log n) is possible
4. **Resource management**: Files, connections, or handles not properly managed
5. **Batch opportunities**: Sequential operations that could be batched
6. **Caching opportunities**: Expensive computations repeated with same inputs

For each finding, provide:
- File path and line numbers
- Description of the efficiency issue
- Concrete fix with example code
- Performance impact estimate (high/medium/low)

Focus argument from user (if any): {focus_argument}

Changed files:
{file_contents}
```

### Step 3: Aggregate Findings

After all three agents complete, collect their results and:

1. **Deduplicate**: Remove findings that overlap across agents (same file, same lines, same core issue)
2. **Prioritize**: Sort by severity/impact - critical and high-impact findings first
3. **Group by file**: Organize findings by file path for efficient fixing
4. **Filter noise**: Discard findings that are subjective style preferences rather than concrete improvements

### Step 4: Present Summary

Show the user a concise summary of findings before applying fixes:

```
## Simplify Review Summary

### Files Reviewed: N files changed

### Findings:
- Code Reuse: X issues (Y high, Z medium)
- Code Quality: X issues (Y critical, Z major)
- Efficiency: X issues (Y high, Z medium)

### Top Issues:
1. [Most impactful finding with one-line description]
2. [Second most impactful finding]
3. [Third most impactful finding]
...

### Applying fixes...
```

### Step 5: Apply Fixes

Apply all fixes directly using the Edit tool. Work through findings file by file, highest priority first.

For each fix:
1. Read the current file state (it may have changed from earlier fixes)
2. Apply the edit
3. Verify the edit was applied correctly

After all fixes are applied, run any available tests or linters to verify nothing was broken:

```bash
# For Rust projects
cargo check 2>&1
cargo test 2>&1

# For Python projects
python -m pytest 2>&1

# For TypeScript projects
npm test 2>&1
```

### Step 6: Final Report

Present the results:

```
## Simplify Complete

### Applied Fixes:
- [X] File: path/to/file.rs - Description of fix
- [X] File: path/to/other.rs - Description of fix
...

### Skipped (manual review needed):
- [ ] File: path/to/complex.rs - Reason it needs manual review

### Verification:
- Tests: PASS/FAIL
- Lint: PASS/FAIL
```

## Handling Edge Cases

- **No changed files found**: Report "No recently changed files found. Specify files or a directory to review."
- **Single file changed**: Run all three reviews on that one file
- **Large number of files (>50)**: Focus on the 20 most recently modified files, note the others were skipped
- **Agent returns no findings**: Report the dimension as clean. "Code reuse: No issues found."
- **Conflicting suggestions**: Prefer the simpler fix. If two agents suggest different fixes for the same code, choose the one that reduces complexity
- **Test failures after fixes**: Revert the specific fix that caused the failure, report it as "needs manual review"

## Integration

This skill works well after:
- Completing a feature branch
- Before opening a PR
- After a large refactoring session
- When preparing code for review

This skill pairs with:
- `/analyze` for deeper philosophy compliance checks
- Code smell detector skill for pattern-specific analysis
- Test gap analyzer for ensuring fixes don't reduce coverage
