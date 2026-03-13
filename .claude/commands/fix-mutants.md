---
description: Fix surviving mutants from a Flawd mutation testing report
allowed-tools: Read, Grep, Glob, Edit, Write, Bash(cargo:*), Bash(npm:*), Bash(pytest:*), Bash(go:*), Bash(python:*)
---

# Fix Surviving Mutants

Read the Flawd handoff index at $ARGUMENTS (default: `flawd-report/handoff/index.json`).

## Process

1. **Read the index** to get the list of files with surviving mutants
2. **Process one file at a time**, starting with the file that has the most survivors
3. For each file, read the corresponding handoff bundle from `by-file/`

## For each surviving mutant

1. Read the **source file** to understand the code around the mutation site
2. Read the **covering test file(s)** listed in the handoff
3. Read the **"What to do"** section for specific fix guidance
4. Add the **minimal test case** needed to kill the mutant
5. After fixing all mutants in a file, **run the test command** to verify your changes pass

## Constraints

- **Only modify test files, never source code** -- the goal is to improve tests, not change behavior
- Add the **minimal test case** needed -- don't over-engineer
- Use the **existing test framework and assertion patterns** in the project
- If a handoff suggests a boundary value test, test the exact boundary
- If multiple mutants are in the same function, consider combining related test cases

## Verification

After fixing mutants in each file:
- Run the project's test suite to verify the new tests pass
- If a test fails, review the fix and adjust

Work through all files in the index until all surviving mutants have been addressed.
