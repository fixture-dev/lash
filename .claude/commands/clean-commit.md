---
description: Use clean-commit skill for git commit with passing checks
---

# Clean Commit Command

This command guides the commit process to ensure high-quality, verified commits that pass all pre-commit checks.

## Core Principles

1. **NEVER use `--no-verify`** - All commits must pass pre-commit hooks
2. **Block on test results** - Run tests in FOREGROUND and wait for completion
3. **Comprehensive resolution** - Fix ALL issues before retrying
4. **Patient iteration** - Continue until successful commit
5. **Clean final state** - End with no staged, unstaged, or untracked changes

## Commit Process

### 1. Pre-Commit Preparation

Before attempting commit:

```bash
# Check current status
git status

# Review what will be committed
git diff --staged
git diff

# Review recent commits for message style
git log -5 --oneline
```

### 2. Stage Changes

Stage all relevant changes:

```bash
# Stage specific files or all changes
git add <files>
# or
git add .
```

### 3. Attempt Commit (FOREGROUND ONLY)

Run commit in foreground to block and wait for all pre-commit checks:

```bash
git commit -m "$(cat <<'EOF'
Your commit message here

EOF
)"
```

<critical-git-practice>
  Do NOT use `run_in_background: true`. The commit must run in foreground.
</critical-git-practice>

### 4. Monitor Pre-Commit Checks (FOREGROUND ONLY)

The pre-commit hook runs these checks:

1. **cargo fmt** - Code formatting (~5 seconds)
2. **cargo clippy** - Linting (~30-60 seconds)
3. **cargo test** - Test suite (~1-5 minutes depending on codebase size)

<critical-git-practice>
  Keep all monitoring in the foreground and patiently wait for completion. The cargo test check may take several minutes depending on test suite size.
</critical-git-practice>

### 5. Handle Check Failures

If ANY check fails:

#### A. Formatting Failures

```bash
# Fix automatically
cargo fmt

# Verify fixes
cargo fmt --check
```

#### B. Clippy (Linting) Failures

```bash
# View warnings/errors
cargo clippy -- -D warnings

# Many clippy issues have auto-fixes
cargo clippy --fix --allow-dirty
```

#### C. Test Failures

- Read ALL test failures from the cargo test output
- Fix each failing test:
  - Read the test file
  - Read the implementation being tested
  - Identify root cause
  - Fix the implementation or test as appropriate
- Run tests locally to verify: `cargo test <test_name>`

### 6. Retry Commit

After fixing issues:

1. Stage the fixes: `git add .`
2. Attempt commit again (foreground, no --no-verify)
3. Wait for all pre-commit checks again
4. Repeat until successful

### 7. Verify Clean State

After successful commit:

```bash
git status
```

Should show:
- "nothing to commit, working tree clean"
- OR only untracked files that should NOT be committed

If there are leftover staged/unstaged changes:
- Review them
- Either commit them (repeat process) or discard them
- Achieve clean `git status`

## Example Workflow

```bash
# 1. Check status
git status

# 2. Stage changes
git add .

# 3. Commit (FOREGROUND - wait for all checks to complete)
git commit -m "$(cat <<'EOF'
Add task parsing improvements

Implements priority 1 optimizations including caching, batch processing,
and incremental updates for better real-time performance.

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"

# 4. If commit fails with test failures:
# - Fix each failing test
# - Stage fixes: git add .
# - Retry commit (step 3)

# 5. Verify clean state
git status
```

## Troubleshooting

### Tests Taking Too Long

- Normal: Several minutes is expected for full test suite
- DO NOT use background execution
- DO NOT use --no-verify to skip
- Be patient and wait for results

### Repeated Test Failures

- Review test output carefully
- Fix root cause, not symptoms
- Run specific tests locally first: `cargo test <test_name> -- --nocapture`
- Ask user for clarification if test expectations are unclear

### Pre-commit Hook Not Running

- Verify hook exists: `ls -la .git/hooks/pre-commit`
- Verify it's executable: `chmod +x .git/hooks/pre-commit`

## Anti-Patterns to Avoid

❌ Using `--no-verify` to skip checks
❌ Running commit in background
❌ Fixing only some test failures and retrying
❌ Committing with unresolved issues
❌ Leaving working directory dirty after commit
❌ Using `&` or background processes for commit
❌ Moving on to other work before commit completes

## Success Criteria

✅ Commit created successfully
✅ All pre-commit checks passed
✅ No test failures
✅ Clean `git status` (no uncommitted changes)
✅ Never used `--no-verify`

## Task Tracking

Use the TodoWrite tool to track the commit process:

1. "Review changes and prepare commit message" - Check git status, diffs, recent commits
2. "Stage changes for commit" - Add files to staging
3. "Attempt commit with pre-commit checks" - Run commit in foreground
4. "Fix [specific issue type] failures" - If checks fail, fix each issue type
5. "Retry commit with fixes" - After fixing, attempt commit again
6. "Verify clean git status" - Ensure working directory is clean

Mark each task as completed only after it successfully finishes. If commit fails, add specific fix tasks before retrying.
