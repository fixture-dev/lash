# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the major version is 0, minor version bumps may contain breaking changes.

## [Unreleased]

## [0.5.0] - 2026-08-15

Lint's output pointed at advice that did not exist: every per-rule code it
printed answered "Unknown error code" when passed to `lash explain`. This
release closes that loop, and stops `W_INDEX_ORPHAN` from reporting files the
root index plainly references.

### Added

- `lash explain` now knows every code `lash lint` emits. The per-rule syntax,
  semantic and cross-file codes (`E_SYNTAX_*`, `E_SEM_*`, `E_NOTE_*`,
  `E_LINK_*`, `W_INDEX_ORPHAN`, and the `W_`/`I_` variants) previously answered
  "Unknown error code", so following the advice in lint's own output was a dead
  end. Warnings and info-level codes are also labelled as such rather than
  introduced as errors.
- Lint's summary names one of the codes it just reported alongside the
  `lash explain` invocation for it.

### Fixed

- `W_INDEX_ORPHAN` no longer fires for files the root index does reference. A
  link destination ended at the last `)` on the line, so an entry annotated
  with a parenthetical — `- [Alpha](tasks/alpha.md) (superseded)` — recorded a
  path matching no file, and only the first link on a line was read at all, so
  two links on one line reported both files as orphans. Destinations now end at
  the parenthesis that closes their own link, parentheses inside a path are
  balanced, angle-bracketed and titled destinations are understood, and every
  link on a line is collected (#60).

- `.lashignore` is reachable from where users hit it. The `W_INDEX_ORPHAN`
  warning names it in the message text, `lash lint --help` and `lash --help`
  describe file discovery, and the README, user guide and error-code reference
  document it. The mechanism already worked; nothing pointed at it (#58).
- `lash explain --list` no longer drops codes whose prefix matched no category.
  Every `W_` and `I_` code was silently missing from the listing.

## [0.4.0] - 2026-08-11

A task's ID is derived from its title rather than stored, so a change to the
derivation rules moves every unpinned ID while every content hash stays
identical — and incremental indexing, which keys off those hashes, never
notices. This release makes the index aware of the rules it was built under,
and adds `lash migrate-ids` to repair the `@depends-on` references a past move
left dangling.

### Added

- `lash migrate-ids` rewrites `@depends-on` references left dangling by a
  task-ID derivation change. It reports by default and writes only with
  `--write`; `--forget` discards the pending renames for a project that would
  rather repair by hand. Only whole references on `@depends-on:` lines are
  rewritten — prose mentioning an old ID is left alone, and so is the
  unqualified `old-id` form, since a bare token can name a file as readily as
  a task.

### Fixed

- Stale task IDs no longer survive `lash index`. A task's ID is derived from
  its title and is not written to the Markdown unless pinned with `@id:`, so a
  release that changes the derivation rules moves every unpinned ID while every
  content hash stays identical — and incremental indexing, which keys off those
  hashes, never re-derives. A file nobody had edited kept serving IDs from
  rules no longer in force: `lash show` printed the stored ID, `lash lint`
  derived a different one and refused to resolve it, and `lash check-index`
  called the index in sync throughout. The index now records the derivation
  version it was built under and re-derives every file when that does not
  match, so an upgrade repairs itself on the next `lash index`. The IDs that
  moved are reported, and recorded for `lash migrate-ids` — the re-derive is
  the only moment both spellings exist. `lash index --force`, which wipes the
  database, salvages the mapping before doing so — it was the workaround people
  reached for, and it used to destroy the one record that could repair the
  references it broke.
- `lash check-index` compares stored task IDs against freshly derived ones
  instead of only comparing content hashes, which by construction cannot see a
  change in how IDs are derived from unchanged content.
- `lash lint` now says when an unresolved reference points at a task ID that a
  derivation change moved, rather than at a task that is missing. Without it
  the error reads as a false positive: the ID it names is exactly the one
  `lash show` prints back.
- `lash add --before/--after` now accept the file-qualified task ID that `lash
  show` and `lash list` print (`index#beta-task`), not just the bare slug. The
  target file is already fixed by `--file`, so the qualifier was redundant, but
  passing it back failed with "task not found" — which read as the task being
  missing rather than the argument being spelled the way lash spells it. A
  qualifier naming a *different* file is still rejected, since that means the
  task was expected somewhere it is not. The not-found error now lists the IDs
  that do exist at that level.
- `lash add --dry-run` now resolves the request instead of echoing it back. It
  never opened the target file, so it reported success for a `--before` naming
  a task that did not exist and the real add then failed on the same argument.
  Dry run and the real add now share one code path, and dry run reports the
  insert line it resolved.

## [0.3.1] - 2026-08-11

Two fixes to the root cause the 0.3.0 sweep left standing. The parsed model
does not record a task's free-text body, and both write paths assumed a task
was its checkbox line plus its annotation block. Both lost content and exited 0.

### Fixed

- `lash add` no longer inserts a new task between the previous task's title and
  its body, which reassigned that body to a task it had nothing to do with.
  Tasks carry prose, numbered steps and acceptance criteria that the parser does
  not record, so the insertion point landed one line below the title. Nothing
  surfaced it: `lash lint` passes on the damaged file, and `lash show` prints a
  task's ID, title, status, file and labels but never its body. Present since
  0.1.0.
- `lash format` no longer deletes what it cannot rebuild from the parsed model.
  0.3.0 stopped it from dropping whole sections, but the `## Tasks` span was
  still regenerated wholesale from the task tree, so free-text bodies, `---`
  separators, comments, `### Subsection` headings and the wrapped continuation
  lines of every contextual note were lost on each run. Formatting
  `lash.index.md` produced a 238-line diff, nearly all of it deletion; it is 52
  lines now. The formatter regenerates the lines the model accounts for and
  copies every other line through.

### Changed

- Blank lines between tasks survive `lash format`. They belong to the author,
  and `lash add` writes one when it appends below a task that has a body.

## [0.3.0] - 2026-08-09

A sweep of the Markdown write path. `lash add` and `lash format` both
regenerated Markdown from a parsed model that did not carry everything the
source did, and neither noticed when the difference cost content. Several of
these fixes address silent data loss that exited 0.

### Added

- The file watcher's outbound channel is bounded at 256 events. Past that the
  debouncer stops sending and raises an overflow flag, and the TUI answers with
  a single full reload instead of thousands of individual reindexes. A branch
  switch across a large project no longer grows memory and latency with the size
  of the burst.

### Changed

- Synthesized task IDs come from one derivation (`synthesize_task_id`) shared by
  `lash add` and the parser, and `lash add` now reports the ID the index will
  actually store. **Task IDs change** for titles containing inline labels or
  punctuation inside a word — a task titled `Ship v0.7.0 release notes #docs`
  was printed as `ship-v0-7-0-release-notes` and stored as
  `ship-v070-release-notes-docs`. Tasks with an explicit `@id:` are unaffected.
  Run `lash check-links` after upgrading to catch `@depends-on` references
  written against a previously printed ID.

### Fixed

- `lash format` no longer deletes sections it does not model. Rebuilding from
  the parsed model dropped every section other than the header, Description and
  Tasks — a file with `## Notes` and `## References` came back with neither, and
  sections above `## Tasks` went too. The formatter now regenerates only the
  spans it owns and copies every other line through unchanged.
- `lash format` no longer appends a duplicate copy of every inline label on each
  run, which left `format --check` reporting a file as dirty forever.
- `lash add` no longer splices a new task into the middle of the preceding
  task's annotation block, which silently truncated multi-line `@agent-note`
  values on the next reindex.
- `lash add` no longer inserts past the end of the annotation block, which
  landed new tasks under a following `## Notes` section, split from the tasks.
- `lash add` into a file with an empty `## Tasks` section no longer prepends the
  checkbox above the H1, where the parser never saw it — the task was written to
  disk while `lash index` reported 0 tasks and `lash lint` passed.
- `lash add --agent-note` with an embedded newline emitted an unindented
  continuation line the parser then dropped. Notes are emitted so they round
  trip, and values that cannot round trip regardless of indentation (a blank
  line, a line starting with `@`) are rejected up front.
- `UserConfig` save/load take an explicit path, so running the test suite no
  longer writes the developer's real `~/.lash/config.toml`. An interrupted run
  previously left `color_scheme = "Test Theme"` behind, which no theme resolves,
  breaking every later `lash` invocation on that machine.

## [0.2.0] - 2026-08-08

### Added

- Homebrew installs via `brew install fixture-dev/tap/lash`. Each release
  publishes a generated formula to the `fixture-dev/homebrew-tap` repo, covering
  macOS and Linux on both x86_64 and arm64.

### Fixed

- File watcher shutdown is now synchronous: dropping the handle signals the
  debouncer, drops the watcher, and joins the thread, so no path can be emitted
  once `drop` returns.

## [0.1.0] - 2026-08-07

Initial release.

### Added

- Markdown task file parser with contextual notes and description sections
- Linter and formatter with 28 validation rules, auto-fix, and interactive mode
- SQLite-backed indexing engine (`lash index`, `lash check-index`)
- Query commands: `lash list`, `lash search`, `lash show`, `lash graph`
- Task mutation commands: `lash add`, `lash complete`, `lash waive`, `lash update`
- Cross-file dependency resolution with cycle detection and `lash check-links`
- Terminal UI (`lash tui`) with 300+ Gogh color schemes and task creation
- Agent integration: `lash agent-prompt` and `lash skill` for coding-agent setups
- Project scaffolding: `lash init` and the PixelQuest `lash playground`
- Configuration management, shell completions, and `lash explain` error catalog

[Unreleased]: https://github.com/fixture-dev/lash/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/fixture-dev/lash/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/fixture-dev/lash/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/fixture-dev/lash/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/fixture-dev/lash/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/fixture-dev/lash/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fixture-dev/lash/releases/tag/v0.1.0
