---
description: Fix surviving mutants from a Flawd mutation testing report
allowed-tools: Read, Grep, Glob, Edit, Write, Bash(flawd:*), Bash(cargo:*), Bash(npm:*), Bash(pytest:*), Bash(go:*), Bash(python:*)
---

# Fix Surviving Mutants

## Argument parsing

The argument provided is: `$ARGUMENTS`

Parse the argument as follows:
- If the argument is a **number between 1 and 100** (e.g., `70`), treat it as a **target mutation score percentage**. Use the default handoff index path `flawd-report/handoff/index.json`.
- If the argument is a **file path**, use it as the handoff index path. No target score — fix all surviving mutants once.
- If no argument is provided, use the default handoff index path `flawd-report/handoff/index.json`. No target score — fix all surviving mutants once.
- If the argument is a number **outside the range 1–100**, stop and report the error: "Target mutation score must be between 1 and 100 (got: <value>)".

## Process

1. **Read the handoff index** to get the list of files with surviving mutants
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

## Iterative mode (target score)

If a **target mutation score** was provided (e.g., `/fix-mutants 70`), follow this loop after completing the initial pass above:

1. **Run `flawd run`** to regenerate the mutation testing report with fresh results
2. **Check the mutation score** in the console output (look for "Mutation score: XX.X%")
3. **If the score meets or exceeds the target** — stop and report success: "Mutation score <actual>% meets target <target>%"
4. **If the score is below the target** — read the new handoff index at `flawd-report/handoff/index.json` and fix the remaining surviving mutants, then return to step 1
5. **If the score does not improve between iterations** — stop and report: "Mutation score plateaued at <score>% (target: <target>%). The remaining mutants may require source code changes or may be equivalent mutants."

Repeat this loop until the target is met or progress stalls. Each iteration should show progress toward the goal.
