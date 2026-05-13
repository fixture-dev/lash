# Agent Skill Install Tasks

**Module:** `lash-agent::installer`, `lash-cli::commands::skill`
**Dependencies:** tasks.agent-integration.md
**Effort:** 1-2 days
**Priority:** HIGH

## Overview

Add a `lash skill install` command that writes Lash agent skills to a
coding-agent's conventional skills directory, using progressive disclosure
(SKILL.md + on-demand reference docs) for Claude Code and single-file
conventions for Codex / Cursor / generic AGENTS.md hosts.

Skill installs are static (project-agnostic knowledge) and complement the
existing dynamic `lash agent-prompt` command, which generates live,
project-specific context on demand. Both share a single source of truth in
`lash-agent::content`.

## Goals

- Reduce token usage in agent context by loading only relevant sub-docs
- Standardise the way Lash gets installed across coding agents
- Stay idempotent so re-running install never overwrites hand-edited files
- Keep skill docs and CLI surface in sync via drift-guard tests

---

## Task 1: Refactor static content into `lash-agent::content` ✅

**Priority:** CRITICAL
**Effort:** 0.5 days
**Status:** COMPLETE

### Description

Extract the static `generate_*_text()` helpers from `prompt.rs` into a
dedicated `content` module so both `agent-prompt` and `skill install` can
compose from the same primitives.

### Subtasks

- [x] Create `crates/lash-agent/src/content.rs` with one `&'static str`
      function per logical section (`overview`, `project_structure`,
      `workflow`, `cli_reference`, `safety_guidelines`, `error_recovery`)
- [x] Port `prompt.rs::build_plain` to consume the new content module
- [x] Publish a canonical `TOP_LEVEL_SUBCOMMANDS` const for drift detection
- [x] Add drift-guard tests in `crates/lash-cli/tests/agent_content_drift_test.rs`
      asserting clap subcommands match `TOP_LEVEL_SUBCOMMANDS` and that
      every user-facing subcommand appears in `cli_reference()`
- [x] Add unit tests covering each content function

### Implementation

See commit `Extract static agent content into lash-agent::content module`.

---

## Task 2: Installer infrastructure + Claude target ✅

**Priority:** CRITICAL
**Effort:** 0.5 days
**Depends on:** Task 1
**Status:** COMPLETE

### Description

Add the installer module that takes a target + scope and writes generated
files to disk, with idempotency markers and user-edit detection.

### Subtasks

- [x] Add `Target`, `Scope`, `InstallOptions`, `InstallReport`,
      `FileAction` types in `lash-agent::installer`
- [x] Implement `install_root()` resolving project- vs user-scope paths
- [x] Implement `generate_files()` per target (Claude only for this task)
- [x] Implement `install()` with idempotency: Created / Updated /
      Unchanged / Skipped / Overwritten
- [x] Implement `plan()` (dry-run wrapper)
- [x] Add the `lash skill <install|list|update|uninstall>` CLI subcommand
- [x] Wire dispatch in `main.rs` and resolve project root
- [x] Add `--force`, `--dry-run`, `--print`, `--scope project|user` flags
- [x] Add unit tests covering all five `FileAction` paths
- [x] Add hot-commands and when-to-use content primitives needed by SKILL.md
- [x] Add dependencies reference content for `references/dependencies.md`

### Implementation

See commit `Add lash skill install command with Claude target`.

---

## Task 3: Codex, Cursor, and AGENTS.md targets ✅

**Priority:** HIGH
**Effort:** 0.5 days
**Depends on:** Task 2
**Status:** COMPLETE

### Description

Add single-file install formats for the remaining agent ecosystems.

### Subtasks

- [x] Implement Codex / AgentsMd shared generator writing
      `AGENTS.lash.md` at the project root (so we never clobber a
      hand-authored `AGENTS.md`)
- [x] Implement Cursor generator writing `.cursor/rules/lash.mdc` with
      Cursor's MDC frontmatter (`description`, `globs`, `alwaysApply`)
- [x] Update `lash skill list` to dedupe by marker path so shared-generator
      targets surface only once
- [x] Add tests verifying file paths, frontmatter, and idempotency for the
      new targets

### Implementation

See commit `Add Codex, Cursor, and AgentsMd skill targets`.

---

## Task 4: Cleanup, docs, task tracking ✅

**Priority:** MEDIUM
**Effort:** 0.25 days
**Depends on:** Task 3
**Status:** COMPLETE

### Description

Remove the placeholder `--format claude-skill` from `lash agent-prompt`
(replaced by `lash skill install --target claude`) and document the new
feature.

### Subtasks

- [x] Remove `AgentFormat::ClaudeSkill` from `lash-cli::cli`
- [x] Remove `PromptFormat::ClaudeSkill` and `build_claude_skill` from
      `lash-agent::prompt`
- [x] Update the config validator's allowed `agent.default_format` values
- [x] Add this task file and link it from `tasks/tasks.md`
- [x] Add a devlog entry summarizing the work

### Implementation

See commit `Remove placeholder --format claude-skill and document skill install`.

---

## Success Criteria

- ✅ `lash skill install --target claude` creates `.claude/skills/lash/SKILL.md`
      and `references/*.md` with the lash-skill-version marker
- ✅ Re-running install is idempotent (Unchanged for files that match)
- ✅ Hand-edited files are preserved unless `--force` is passed
- ✅ `--dry-run` and `--print` emit the plan without writing
- ✅ `lash skill list`, `update`, and `uninstall` are wired
- ✅ All four targets work; non-Claude targets emit a single-file install
- ✅ Drift-guard tests catch new CLI subcommands that miss the agent docs
- ✅ Workspace builds, clippy clean, all tests passing

## Tests

- Unit tests in `lash-agent::content::tests` and
  `lash-agent::installer::tests` (18 tests covering all five FileActions and
  all four targets)
- Drift-guard integration test in
  `crates/lash-cli/tests/agent_content_drift_test.rs`
- Snapshot test on `agent-prompt` output continues to pass with the
  refactored content

## Notes

- Codex and AgentsMd targets currently share one generator. If they ever
  diverge (e.g. Codex picks up a distinct file format), split the
  generators rather than parameterising one.
- The skill-version marker (`lash-skill-version: <CARGO_PKG_VERSION>`) is
  stamped into every generated file. Hand-edited files lack it and are
  preserved across re-installs.
- Cursor support is best-effort; the MDC `globs` list is reasonable but
  not authoritative — adjust if Cursor's spec changes.
