---
name: batch
description: Orchestrate large-scale changes across a codebase in parallel
disable-model-invocation: true
---

# Batch Skill

## Purpose

Orchestrate large-scale changes across a codebase by decomposing the work into independent units and executing them in parallel. Each unit runs in an isolated git worktree with its own agent, implements the change, runs tests, and opens a PR.

## Prerequisites

- Must be in a git repository
- Must have a clean working tree (no uncommitted changes)
- Must have a remote configured for PR creation
- `gh` CLI must be available for creating pull requests

Verify prerequisites before proceeding:

```bash
git rev-parse --is-inside-work-tree 2>/dev/null || echo "NOT_A_GIT_REPO"
git status --porcelain | head -5
git remote -v | head -2
which gh 2>/dev/null || echo "GH_CLI_NOT_FOUND"
```

If any prerequisite fails, stop and report what is missing. Do not proceed without a git repository.

## When This Skill Activates

- User invokes `/batch <description of large-scale change>`
- User asks to "batch", "make changes across the codebase", or "apply everywhere"

## Input

- **Required**: Description of the change to apply across the codebase
- Examples:
  - "Add structured logging to all HTTP handlers"
  - "Migrate all database queries from raw SQL to the query builder"
  - "Add input validation to all public API endpoints"
  - "Update all error types to include error codes"
  - "Add OpenTelemetry tracing spans to all service methods"

## Process

### Phase 1: Research the Codebase

Before decomposing work, understand the codebase structure and the scope of the requested change.

1. **Identify relevant files**: Use Grep and Glob to find all files affected by the change
2. **Understand patterns**: Read representative examples to understand existing patterns and conventions
3. **Map dependencies**: Identify shared types, traits, modules, or utilities that units may depend on
4. **Detect conflicts**: Identify files or modules that multiple units might need to modify (these cannot be parallelized)

```bash
# Example: finding all HTTP handlers
grep -rl "fn handle\|async fn handle\|#\[handler\]\|@app.route\|router\." src/

# Example: understanding the project structure
find . -name "*.rs" -o -name "*.py" -o -name "*.ts" | head -50
```

### Phase 2: Decompose into Independent Units

Break the change into 5-30 independent units. Each unit must be:

- **Independent**: Can be implemented without knowledge of other units
- **Self-contained**: Changes only files that no other unit touches
- **Testable**: Has a clear way to verify correctness
- **Small**: Ideally touches 1-5 files

For each unit, define:

```
Unit N: [Short title]
- Files to modify: [list of file paths]
- Description: [What to change and how]
- Test command: [How to verify this unit works]
- Estimated complexity: simple | moderate | complex
```

Rules for decomposition:
- **No overlapping files**: Two units must never modify the same file. If they must, merge them into one unit.
- **Shared dependencies first**: If the change requires a new shared type, trait, or utility, that must be Unit 1 (implemented first, not in parallel).
- **Group by module**: Prefer grouping related files into the same unit over splitting them
- **Respect module boundaries**: Each unit should ideally stay within one module or package

### Phase 3: Present Plan for Approval

Present the decomposition to the user and wait for explicit approval before proceeding.

```
## Batch Change Plan

### Change: [User's requested change]

### Research Summary:
- Total files affected: N
- Modules involved: [list]
- Shared dependencies needed: [list or "none"]

### Decomposition: N units

**Unit 1: [Title]** (simple)
- Files: path/to/file1.rs, path/to/file2.rs
- Change: [one-line description]
- Test: cargo test -p module_name

**Unit 2: [Title]** (moderate)
- Files: path/to/file3.rs
- Change: [one-line description]
- Test: cargo test -p other_module

...

### Execution Order:
- Sequential first: [Units that must go first, e.g., shared type definitions]
- Parallel batch: [Units that can run simultaneously]

### Estimated total: N units, ~M minutes

Proceed? (yes/no)
```

**Do not proceed without user approval.** This is the one point where the skill must stop and wait. The user may want to adjust units, remove some, or change the approach.

### Phase 4: Execute Units in Parallel

After approval, execute the units. Sequential units (if any) run first, then parallel units launch simultaneously.

#### For Each Unit: Spawn a Background Agent

Each unit runs as an independent background agent via the Task tool. Each agent receives:

1. The unit description and file list
2. Instructions to work in an isolated git worktree
3. The test command to run after implementation
4. Instructions to open a PR when done

Agent prompt template for each unit:

```
You are implementing one unit of a batch change.

## Setup

Create an isolated git worktree for this unit:

```bash
BRANCH_NAME="batch/{unit_slug}"
WORKTREE_DIR="/tmp/batch-{unit_slug}-$(date +%s)"

git worktree add "$WORKTREE_DIR" -b "$BRANCH_NAME" main
cd "$WORKTREE_DIR"
```

## Your Task

Unit: {unit_title}
Files to modify: {file_list}
Description: {unit_description}

## Implementation Rules

1. Only modify the files listed above
2. Follow existing code patterns and conventions in the repository
3. Do not introduce new dependencies unless absolutely necessary
4. Keep changes minimal and focused on the described task
5. Add or update tests if the change affects behavior

## After Implementation

1. Run the verification command:
   ```bash
   {test_command}
   ```

2. If tests pass, commit and push:
   ```bash
   git add {file_list}
   git commit -m "batch: {unit_title}"
   git push -u origin "batch/{unit_slug}"
   ```

3. Create a PR:
   ```bash
   gh pr create \
     --title "batch: {unit_title}" \
     --body "Part of batch change: {overall_description}\n\nUnit {unit_number} of {total_units}." \
     --base main
   ```

4. If tests fail, fix the issue and retry. If you cannot fix it after two attempts, commit what you have with a `[WIP]` prefix on the PR title and note what failed.

## Cleanup

After pushing (success or failure):
```bash
cd /home/azureuser/src/RustyClawd
git worktree remove "$WORKTREE_DIR" --force 2>/dev/null
```
```

### Phase 5: Monitor and Report

After all agents are dispatched, monitor their progress. As each agent completes, collect its result.

Present a rolling status table:

```
## Batch Progress

| Unit | Status | PR | Notes |
|------|--------|----|-------|
| 1. Add logging to auth handlers | Done | #123 | Tests pass |
| 2. Add logging to user handlers | Done | #124 | Tests pass |
| 3. Add logging to payment handlers | Running | - | - |
| 4. Add logging to notification handlers | Running | - | - |
| 5. Add logging to admin handlers | Queued | - | - |
```

### Phase 6: Final Summary

After all units complete, present the final summary:

```
## Batch Complete

### Overall: {succeeded}/{total} units succeeded

### PRs Created:
- #123: batch: Add logging to auth handlers
- #124: batch: Add logging to user handlers
- #125: batch: Add logging to payment handlers
...

### Failed Units (manual attention needed):
- Unit 5: Add logging to admin handlers - Test failure in test_admin_audit
  Branch: batch/admin-handler-logging (WIP PR #127)

### Cleanup:
- All worktrees removed
- {N} branches pushed to remote

### Next Steps:
- Review and merge PRs individually or as a batch
- Address any failed units manually
```

## Handling Edge Cases

- **Fewer than 5 units**: Proceed normally. Even 2-3 units benefit from parallel execution.
- **More than 30 units**: Group related units to reduce below 30. Too many parallel agents degrades performance.
- **Shared file conflict detected during decomposition**: Merge the conflicting units into one. Never allow two units to touch the same file.
- **Agent fails to create worktree**: Fall back to working on a branch directly (without worktree isolation). Note this in the status.
- **Test infrastructure missing**: If no tests exist for a unit, the agent should still implement the change and note "no tests available" in the PR.
- **User rejects plan**: Ask what to adjust. Re-decompose if needed.
- **Repository has no remote**: Skip PR creation. Commit to local branches only and report branch names.
- **gh CLI not available**: Skip PR creation. Push branches and report them. User can create PRs manually.

## Constraints

- This skill orchestrates work; it does not implement changes directly
- Each unit agent is fully autonomous once dispatched
- Units must be truly independent - no shared mutable state between parallel units
- The skill must wait for user approval of the plan before executing
- Failed units do not block successful ones
- All worktrees must be cleaned up, even on failure

## Integration

This skill works well for:
- Codebase-wide migrations (API changes, library upgrades)
- Applying consistent patterns across modules (logging, error handling, tracing)
- Bulk refactoring (renaming, restructuring)
- Adding cross-cutting concerns (validation, authentication checks)

This skill pairs with:
- `/simplify` for post-batch cleanup of each unit
- Test gap analyzer for ensuring batch changes have adequate coverage
- PR review assistant for reviewing the generated PRs
