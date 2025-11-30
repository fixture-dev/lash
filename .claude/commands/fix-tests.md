---
description: Fix failing tests comprehensively with foreground execution
argument-hint: "[test-names]"
allowed-tools: Bash(cargo:*), Read, Grep, Glob, Edit, Write
---

If specific test names were provided as an argument ($1):
Please engage the appropriate subagents to comprehensively fix these failing tests: $1

If no argument was provided:
Please engage the appropriate subagents to run `cargo test` and comprehensively fix all resulting failures.

Important execution requirements:
- Always run tests in the FOREGROUND (never in background)
- Patiently poll for test results and wait for completion
- Iterate on fixes until achieving robust and lasting solutions for all failures
- Do not move on until all tests pass consistently
- Address root causes, not symptoms
