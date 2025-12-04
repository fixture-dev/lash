# Lash Design Document (v0.1)

> Minimalist, ultra-fast, Markdown-native task tracker for devs and agents.

---

## 1. Purpose & Scope

Lash is a terminal-first task tracker where **Markdown is the source of truth** and everything else is an acceleration layer.

It is intended to be:

* **Minimalist** – few moving parts, opinionated format.
* **Ultra-fast** – optimized parsing, indexing, and search.
* **Markdown-native** – all task data lives in a strictly formatted Markdown tree.
* **Agent-friendly** – trivial for LLM agents (Claude Code, etc.) to use safely.

This document defines the **concepts, data model, file formats, commands, and architecture** so that we can derive concrete implementation tasks from it.

This is a **living document**: sections marked as “TBD/Decide” are design surfaces to refine, not blockers.

---

## 2. Goals & Non-Goals

### 2.1 Goals

1. **Markdown as the single source of truth**

   * All tasks/subtasks live in `.md` files.
   * The SQLite layer is fully reconstructible from the Markdown tree.

2. **Strict, linter-enforced format**

   * Predictable for agents.
   * Predictable for tools and code.
   * Enforced by a **fast Rust linter** accessible via CLI and TUI.

3. **Dependency graph over a tree**

   * Within a file: checkbox hierarchy.
   * Across files: directory structure + explicit links.
   * A task or file “completes” only when its dependent children are complete or explicitly waived.

4. **Terminal-first UX**

   * Single binary CLI for:

     * Linting, formatting, indexing.
     * Querying tasks.
     * Generating agent usage prompts.
   * Simple TUI for browsing, filtering, and editing tasks (via external `$EDITOR` or inline operations).

5. **Cross-cutting labels & fast search**

   * Labels for dynamic slices (e.g. `#backend`, `#infra`, `#agent`).
   * Fuzzy search across task titles, bodies, labels, filenames, and references.

6. **Agent-oriented design**

   * Easy to generate **context-minimized prompts** for LLMs.
   * Data structures & algorithms tuned to reduce token usage.
   * Rich, structured errors that are understandable both by humans and agents.

### 2.2 Non-Goals (v1)

* Multi-user synchronization / shared server.
* Complex workflow automation (e.g. Kanban, burndown charts).
* Rich formatting beyond what is needed for tasks and annotations.
* Web UI (CLI/TUI only in v1).

---

## 3. Core Concepts

### 3.1 Project Root & Index

* A Lash repo lives under a filesystem root.
* A **root index Markdown file** (e.g. `lash.index.md` or `index.lash.md`) exists at the root.
* The root index:

  * Provides a **nested outline of the directory structure**.
  * Points to topic files.
  * Optionally stores project-wide metadata (title, description, global tags).

### 3.2 Topic Files

* A **topic file** is a Markdown file containing a **depth-limited hierarchical checklist** plus annotations.
* Examples:

  * `photo-app.filters.sepia.md`
  * `core.api.auth.md`
* Each topic file:

  * Encodes a subtree of tasks.
  * May reference other topic files.
  * Forms part of the dependency graph.

### 3.3 Directories & Naming

* Directory names use the same **dot-delimited topical naming convention** as files.
* Filenames reflect their place in the conceptual graph:

  * `photo-app/filters/sepia.md`
  * Or `photo-app.filters/sepia.md`
  * Or pure “flat with dot-paths” like `photo-app.filters.sepia.md`.
* **Decision**: Lash treats **path + filename** as the canonical task “namespace”. Dot-conventions help agents/humans see conceptual structure at a glance.

### 3.4 Tasks & Dependencies

* Tasks are primarily expressed as **checkbox items** with optional metadata.

* Dependencies:

  * **Within file**: parent checkbox depends on its nested child checkboxes.
  * **Across files**: files depend on the completion of:

    * Their direct subdirectories.
    * Any tasks explicitly referenced via links.

* A task/file is:

  * **Incomplete** until all dependencies are `done` or `waived`.
  * **Blocked** if any dependency is incomplete and not waived.

### 3.5 Labels

* Labels are **lightweight tags** attached to tasks.
* Modeled either as:

  * Inline markers (e.g. `#backend`, `#agent`, `#rust`).
  * Or a structured annotation line (`@labels: backend, agent`).
* Labels support:

  * Cross-cutting slices (e.g., “all tasks `#agent` for the coding agent”).
  * Agent prompt generation (“give me all `#high-priority` tasks for this agent”).

### 3.6 Acceleration Layer

* A **SQLite database** stores:

  * Parsed tasks.
  * Dependency edges.
  * Labels.
  * File metadata (paths, hashes, mtime).
  * Search indexes.
* It is **rebuildable** from the Markdown tree at any time.

---

## 4. Markdown File Format

### 4.1 High-Level Structure

Each task file has:

1. **Header block** (YAML-like or fenced metadata; exact syntax TBD but consistent).
2. **Description section** (optional but recommended) - Free-form Markdown text explaining the scope, context, and intent of the file.
3. **Task tree** (hierarchical checkbox list).
4. **Optional reference/notes section**.

Example sketch (**not final syntax**, but concrete enough to implement):

```markdown
# Photo App – Sepia Filter

@id: photo-app.filters.sepia
@labels: photo-app, filters, image-processing
@status: in-progress
@owner: frank
@created: 2025-11-16

## Description

This file tracks implementation of the sepia filter effect for the photo app.
The sepia filter applies a warm, vintage tone to images by shifting RGB values
toward brown/tan colors. @agent-note: This is a non-destructive operation that
should preserve the original image data.

Key constraints: must process 4K images in under 100ms, integrate with existing
filter pipeline, support both CPU and GPU implementations.

## Tasks

- [ ] Implement sepia filter core
  - [ ] Define parameter schema
  - [ ] Write Rust core function
  - [ ] Add tests
- [ ] Integrate with UI
  - [ ] Wire up settings panel
  - [ ] Hook into preview
- [ ] Performance & QA
  - [ ] Benchmark on sample images
  - [ ] Fix regressions

## References

- Depends on: `../core/image-pipeline.md`
- Related: `../photo-app.filters.vignette.md`
```

#### Description Section Specification

The `## Description` section:

* **Placement**: Must appear after the metadata block and before `## Tasks`.
* **Purpose**: Provides contextual information about the file's scope, goals, constraints, and implementation notes. This helps both humans and agents understand the purpose and context before diving into tasks.
* **Content**: Free-form Markdown text. May include:
  * Overview of what this file covers
  * Key constraints or requirements
  * Design decisions or architectural notes
  * Implementation guidance
  * Inline `@agent-note:` annotations for LLM-specific hints
* **Length limits**:
  * Recommended: 500-1000 characters
  * Warning threshold: 1000 characters (linter will warn)
  * Error threshold: 2000 characters (linter will fail)
  * Rationale: Keeps descriptions concise and agent-token-friendly while allowing sufficient context.
* **Optional but recommended**: While not strictly required, the linter may suggest adding a description section for files with significant task trees or complex dependencies.

### 4.2 Task Line Format

Canonical task line grammar (conceptual):

```
TASK_LINE := INDENT* "- [STATUS] " TITLE (METADATA_BLOCK?) (INLINE_LABELS?)
```

Where:

* `STATUS` is one of:

  * `[ ]` = open
  * `[-]` = waived/not applicable
  * `[x]` = done
  * `[!]` = blocked (optional extension)
* `METADATA_BLOCK` might be a trailing structured element like:

  * `[@id: sepia-core, @labels: filters,backend, @estimate: 2h]`
* `INLINE_LABELS`:

  * `#backend #filters #agent`

#### Depth Limitation

* Maximum depth, e.g. **3 or 4 levels** (`-`, `  -`, `    -`, `      -`).
* Enforced by linter to keep trees shallow and agent-friendly.

### 4.3 Special Annotation Types

A small, fixed set of annotations, for example:

* `@id: <string>` – unique within the file; combined with file path to form a globally unique task ID.
* `@labels: a, b, c`
* `@owner: <string>`
* `@status: <overall status for the file>`
* `@estimate: <duration>` (for optional planning).
* `@depends-on: <path or id>` (explicit extra dependencies).
* `@agent-note:` – a short note addressed to agents (“LLM-specific hints”).

These appear in either:

* The header, e.g. file-level metadata.
* Inline with tasks (inside a trailing metadata block).

The linter ensures:

* No unknown annotation tags.
* Correct formats.

---

## 5. Dependency Model

### 5.1 Within a File

* Parent tasks are **implicitly dependent** on all of their descendant tasks.
* Rules:

  * A non-leaf task cannot be `done` while any descendant is `open` or `blocked`.
  * A descendant may be `waived` (`[-]`) to explicitly mark it as unnecessary.

### 5.2 Across Files – Directory Graph

* Let `D` be a directory.
* Let `child_files(D)` be all topic files directly under `D`.
* Let `child_dirs(D)` be all subdirectories under `D`.
* Rule:

  * A topic file in directory `D` is considered **conceptually dependent** on the *complete closure* of `child_dirs(D)` **if** the root index defines such a relationship.
  * This is encoded either:

    * Implicitly: directory-level semantics.
    * Or explicitly: via annotations in the root index or directory metadata file (e.g. `@depends-on-dirs: filters, core`).

### 5.3 Explicit Path / ID Dependencies

* Tasks can explicitly reference other tasks/files:

Examples:

```markdown
@depends-on: ../core/image-pipeline.md#task:normalize-frames
@depends-on: photo-app.filters.base#task:filter-registry
```

* The linter and accelerator:

  * Resolve these references.
  * Track them as graph edges.
  * Detect broken links.

### 5.4 Completion Semantics

* **Task is complete** when:

  * Status `done`, and
  * All descendants are `done` or `waived`, and
  * All explicit dependencies (by ID/path) are complete or waived.

* **File/topic is complete** when:

  * All top-level tasks are complete, and
  * Any configured directory-level dependencies are complete.

---

## 6. Root Index Format

The root index file gives a **coarse map** of the project.

Example:

```markdown
# Lash Task Index

@project: Lash
@version: 0.1

## Core

- [ ] `core/architecture.md`
- [ ] `core/cli.md`
- [ ] `core/tui.md`

## Acceleration Layer

- [ ] `accel/indexing.md`
- [ ] `accel/sqlite-schema.md`

## Agents

- [ ] `agents/usage.md`
- [ ] `agents/prompt-templates.md`
```

The linter enforces:

* Paths exist.
* No cycles in this high-level reference layer (or cycles are detectable and surfaced).

---

## 7. CLI Design

### 7.1 Binary

* Single binary: `lash`
* Built in Rust as a static or mostly static binary where possible.
* Cross-platform: macOS, Linux, Windows.

### 7.2 Global Behavior

* `lash` commands always run relative to a **project root**:

  * Either inferred (search upward for `lash.index.md` or `.lash/`).
  * Or explicitly specified via `--root`.

* Exit codes & messages designed to be:

  * Human friendly.
  * Agent friendly (optional `--json` output).

### 7.3 Core Commands (v1)

**1. Linting & Formatting**

* `lash lint [PATH...]`

  * Validate Markdown format.
  * Validate annotations and task structures.
  * Validate links and IDs.
  * Options:

    * `--json`: machine-readable diagnostics.
    * `--fix`: auto-fix simple issues (whitespace, ordering, etc.).

* `lash format [PATH...]`

  * Normalize indentation, ordering of annotations, etc.

**2. Indexing & DB Operations**

* `lash index`

  * Walk the project tree.
  * Parse all Markdown files.
  * Rebuild the SQLite acceleration layer from scratch.

* `lash check-index`

  * Verify that DB is consistent with Markdown.
  * Optionally show drift.

**3. Querying**

* `lash list [FILTERS]`

  * List tasks matching criteria:

    * `--label backend`
    * `--status open`
    * `--path core/`
    * `--blocked`
    * `--agent <name>` (based on owner/labels).

* `lash search <QUERY>`

  * Fuzzy search against titles, bodies, and labels.

* `lash show <TASK_ID_OR_PATH>`

  * Render the canonical view of a specific task/file, including dependencies and status.

**4. Graph & Links**

* `lash graph [--format=dot] [--scope=path_or_label]`

  * Output dependency graph (e.g., for `dot`).

* `lash check-links`

  * Scan for:

    * Broken `@depends-on` references.
    * Broken file paths in index.

**5. Agent Prompt Generation**

* `lash agent-prompt --format=plain|json|claude-skill|agents-md [OPTIONS]`

  * Generate text that explains:

    * How the task system is structured.
    * How to safely:

      * Add tasks.
      * Mark tasks done.
      * Add dependencies.
  * Context-minimized:

    * Only include the essential schema, examples, and allowed operations.
  * Options:

    * `--for-owner <name>`: include relevant tasks for that agent.
    * `--include-examples`: embed exemplar task files.

**6. Maintenance Utilities**

* `lash archive [OPTIONS]`

  * Move completed/obsolete tasks into an archive directory.
  * Update index and links (where possible).

* `lash reindex`

  * Alias for `lash index`.

* `lash fix-links`

  * Attempt to auto-resolve broken links based on fuzzy matching, optionally interactive.

**7. TUI Launcher**

* `lash tui`

  * Start TUI interface (see section 8).

---

## 8. TUI Design

The TUI focuses on **readability & velocity**, not fancy UI.

### 8.1 Layout

* Left pane: navigation tree (directories, files, labels).
* Right pane: current topic file with:

  * Task list.
  * Status summary.
  * Dependencies overview.

### 8.2 Interactions

* Keyboard-centric:

  * `j/k` or arrows: move selection.
  * `space`: toggle task status.
  * `e`: open file in `$EDITOR`.
  * `/`: fuzzy search.
  * `l`: filter by label.
  * `g`: view dependency graph summary for selected task/file.
  * `?`: help overlay.

### 8.3 Agent Awareness

* TUI can show a **“Agent view”**:

  * Highlight tasks tagged with `#agent` or owned by an agent.
  * Summarize tasks into a short bullet list that matches token budgets.

---

## 9. Acceleration Layer & SQLite Schema

### 9.1 Principles

* **Source of truth**: Markdown.
* **DB**: ephemeral and fully reconstructible.
* **Performance**:

  * Optimize for:

    * `lint + index` on medium-large repos.
    * Interactive search and filtering.

### 9.2 Suggested Schema (First Pass)

Tables (names illustrative):

* `files`

  * `id` (PK)
  * `path` (unique)
  * `hash` (content hash, e.g. blake3)
  * `mtime`
  * `status` (overall file state)
  * `description` (TEXT; content from `## Description` section)
  * `labels` (optional normalized relation or JSON)
  * `meta` (JSON blob for extra header info)

* `tasks`

  * `id` (PK; internal integer)
  * `file_id` (FK)
  * `local_id` (string from `@id` or synthesized)
  * `full_id` (unique `path#local_id`)
  * `title`
  * `status` (open, done, waived, blocked)
  * `depth`
  * `parent_id` (FK to tasks.id for hierarchy)
  * `order_index`
  * `labels` (optional relation or JSON)
  * `owner`
  * `estimate`
  * `body` (optional long text for detail)

* `dependencies`

  * `from_task_id`
  * `to_task_id`
  * `kind` (implicit-hierarchy, explicit-id, explicit-path, directory)

* `labels`

  * `id`
  * `name`

* `task_labels`

  * `task_id`
  * `label_id`

* `file_labels`

  * `file_id`
  * `label_id`

Indexes:

* On `tasks.status`, `labels.name`, `files.path`.
* FTS (if used) on `tasks.title`, `tasks.body`, `files.path`, `files.description`.

### 9.3 Fuzzy Search

Two possible approaches (can be evaluated experimentally):

* SQLite FTS5 for full-text search.
* Lightweight in-Rust fuzzy matcher (e.g. building a simple index in memory from DB rows).

In both cases, the DB stores the canonical data; fuzzy search runs on top.

---

## 10. Linter Design

### 10.1 Responsibilities

* Validate **syntax**:

  * Correct Markdown checkbox patterns.
  * Indentation rules.
  * Annotation formats.

* Validate **semantics**:

  * Unique `@id` within file.
  * Proper depth limits.
  * No unknown annotation keys.
  * Correct `@depends-on` resolution (optionally using DB if available).
  * Status consistency: parents vs children.

* Provide **fixes** where safe:

  * Normalize indentation.
  * Sort annotations.
  * Add missing header boilerplate.

### 10.2 Performance

* Implemented in Rust using:

  * Handwritten or lightweight parser (no overkill).
* Operates in:

  * **Single-file mode**: for pre-commit hooks.
  * **Project mode**: integrated with `lash index`.

### 10.3 Diagnostics

* Human-readable default:

  * File path, line/col, short message.

* Machine-readable:

  * `--json` with:

    * `code` (stable diagnostic ID).
    * `severity`.
    * `message`.
    * `location`.
    * Optional `suggested_fix`.

This allows agents to:

* Detect issues.
* Offer auto-fix workflows.

---

## 11. Agent Integration & Token Minimization

### 11.1 Usage Model

The typical agent (e.g., Claude Code) will:

1. Call `lash agent-prompt ...` to get instructions and optionally a task subset.
2. Read/modify specific Markdown files.
3. Call `lash lint` to validate changes.
4. Possibly call `lash index` or incremental indexing commands after larger updates.

### 11.2 Token Minimization Strategies

1. **Schema-first prompts**

   * Provide a **minimal, stable schema** for the task format.
   * Avoid embedding large samples unless requested.
   * Use small, representative snippets as examples.

2. **Sparse context**

   * For a given agent action, include:

     * The specific topic file being edited.
     * Its immediate parents/children (just headers & statuses).
     * A short textual summary (generated by Lash) of relevant dependencies:

       * e.g. “This task depends on 3 tasks in `core/image-pipeline.md` (all done).”

3. **Summarization layer**

   * Lash can optionally store auto-generated **summaries** of large files:

     * Short bullet list of major tasks and status.
   * For agents, only the summary + specific task snippet is included.

4. **ID-based references**

   * Encourage agents to refer to tasks by `full_id` rather than copying entire descriptions into the prompt.

5. **Compact formats**

   * When generating JSON for agents (e.g. `--format=json`), keep keys short but stable.
   * Allow agents to request **“schema-only mode”** vs “schema+examples”.

### 11.3 Prompt Generation Command

`lash agent-prompt` should be able to output:

* **Plain text** instructions explaining:

  * Allowed operations.
  * Format rules.
  * Error-handling expectations.

* **Claude Code skill spec**:

  * A JSON/YAML spec containing:

    * Command names.
    * Arguments.
    * Example invocations.

* **Agents.md fragment**:

  * A ready-to-paste Markdown block describing:

    * “How Lash works.”
    * “How to use Lash safely as an agent.”

---

## 12. Error Handling & UX

### 12.1 Principles

* Errors are:

  * **Expressive** – clearly explain what went wrong and how to fix it.
  * **Structured** – stable machine-readable codes and fields.
  * **Non-panicky** – prefer “soft” errors where possible with guidance.

### 12.2 Examples

* “Broken dependency”:

  * Human:

    > `core/cli.md:42: Unknown dependency '@depends-on: core/parser#task:xyz' (target not found).`

  * JSON:

    ```json
    {
      "code": "E_DEP_NOT_FOUND",
      "file": "core/cli.md",
      "line": 42,
      "dependency": "core/parser#task:xyz",
      "suggestion": "Check that the target file exists and contains a task with this id."
    }
    ```

* “Depth limit exceeded”:

  * Human:

    > `core/architecture.md:57: Task depth (5) exceeds configured maximum (3). Consider flattening or splitting the file.`

### 12.3 Exit Codes

Standardized exit codes, e.g.:

* `0` – success.
* `1` – general error.
* `2` – lint errors.
* `3` – index errors.
* `4` – configuration or project root issues.

---

## 13. Architecture & Implementation Plan (High-Level)

### 13.1 Rust Project Structure

Suggested crate/module boundaries:

* `lash-cli`

  * CLI parsing (e.g. `clap`).
  * Integrates all subsystems.

* `lash-core`

  * Markdown parser.
  * Task model.
  * Linter.
  * Dependency resolution.

* `lash-db`

  * SQLite schema & migrations.
  * Indexing and query layer.

* `lash-tui`

  * TUI implementation (e.g. based on `crossterm` + `ratatui` or equivalent).

* `lash-agent`

  * Prompt generation.
  * Token minimization utilities.
  * JSON schema definitions for agents.

### 13.2 Performance Considerations

* Use incremental indexing where possible:

  * Track `mtime` and hash of files.
  * Only re-parse on change.
* Optionally use `PRAGMA` tuning in SQLite (WAL mode, synchronous options) if safe for the use case.

---

## 14. Extensibility & Future Directions

Potential future features (beyond v1):

* Per-user configuration file for:

  * Depth limits.
  * Default labels.
  * Agent-specific settings.
* Templates for creating new topic files.
* Integration with external tools:

  * Git hooks (`pre-commit` lint).
  * Editors (VS Code, Neovim) via a language server or simple CLI integration.
* More sophisticated archiving / history:

  * Optional commit snapshots of completed tasks.

---

## 15. Open Design Topics / Decisions (Non-blocking)

These don’t need answers before starting v1 but should be decided as we iterate:

* Exact syntax of the header block (YAML frontmatter vs inline `@` annotations only).
* Concrete maximum depth for task trees (3 vs 4).
* Preferred canonical naming scheme:

  * `photo-app.filters.sepia.md` in a flat directory vs nested directories.
* Choice of crates for:

  * TUI.
  * Fuzzy search (native vs SQLite FTS).
* Whether to support inline rich Markdown (code blocks, tables) inside task bodies in v1.

---

