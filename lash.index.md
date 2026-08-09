# Lash

@id: lash
@labels: rust, cli, tui
@created: 2026-05-22

## Description

Lash is a minimalist, ultra-fast, Markdown-native task tracker for devs and
agents. This `lash.index.md` is the project's own dogfooding index — it
tracks the high-level engineering work that surfaces in `lash status`,
`lash tui`, etc.

The bulk of the historical task breakdown lives under `tasks/` in a
documentation-style format that predates Lash's own task-file linter. Those
files aren't indexed by Lash today (they're excluded via `.lashignore`); the
live tracker here in `lash.index.md` is for current and near-future work
that should be visible through Lash itself.

For background docs, see:

- [docs/design-doc.md](docs/design-doc.md)
- [docs/live-tui-updates.md](docs/live-tui-updates.md)
- [devlog.md](devlog.md)

## Tasks

### Live TUI updates

- [x] Design live-TUI-updates architecture #design
- [x] Activity status bar (in-progress + recently completed) #tui
- [x] Backfill activity bar from DB at startup so it's not empty on first launch #tui
  - `ActivityState::seed_from_db` now seeds both the in-progress slot and
    the recently-completed tail from `TaskRepository`. The TUI no longer
    looks broken when the user has no live transitions yet — the bar
    populates with up to 3 done/waived tasks from files modified in the
    last 5 min, ordered by file mtime DESC
- [x] Store actor with atomic writes and hash dedupe #core
- [x] File watcher with debounce and ignore rules #core
- [x] External-edit reload with stable-id cursor preservation #tui
- [x] Parse-and-diff external changes into `TaskStatusChanged` deltas #core #tui
  - The activity bar now updates from external edits too: `handle_file_reloaded`
    snapshots the file's task statuses before reindex and feeds every changed
    status through `ActivityState::record_transition` afterwards
- [x] `Mutation::CreateTask` so task creation also flows through the Store #core
  - TUI's submit handler now calls `Store::apply(CreateTask)`. The Store
    delegates emission to `TaskCreationService`, re-reads the resulting
    file, and records its hash so the watcher's echo gets dedupe'd — no
    more redundant reindex on a TUI-initiated create
- [x] `formatter::format_file_in_place` uses `write_atomic` #core
  - `lash format` (both the library helper and the CLI's inline write
    path) now go through `lash_core::store::write_atomic` so a crash
    mid-write can't leave a half-formatted Markdown file on disk
- [x] Stale-modal banner for in-flight modals on external change #tui
  - Task-creation modal gets a `stale` flag; external edit to its
    `target_file` flips it. Submit refuses, modal title and border turn
    warning-colored. User Esc-discards and retries against fresh state.
    The other (transient) confirm modals are not yet covered — typical
    open-window is sub-second so the risk window is much smaller
- [x] Bounded watcher channel with `FullReload` overflow path #core
  - The watcher's outbound channel is a `sync_channel` of 256. Past that the
    debouncer stops sending and raises an overflow flag instead of queueing;
    `WatcherEvents::drain` hands the consumer paths and the flag together, so
    a partial path list cannot be read as a complete one. The TUI answers an
    overflow with `Store::handle_watcher_overflow`, which clears the
    self-write hash table and emits `StateDelta::FullReload`, and reindexes
    the whole project instead of acting on the subset
  - Sends never block: the debouncer thread is also what observes the shutdown
    flag, so parking it on a full channel would make `drop` wait on a consumer
    that may itself be waiting

### CLI hygiene

- [x] Cap project-root search at the enclosing git repository #cli
- [x] Consolidate the four `find_project_root` implementations into one #cli #refactor
  - Single canonical walker now lives in `lash_types::path_utils::find_project_root_from`
    plus a shared `is_project_root_marker` predicate. The four pre-existing
    finders are thin wrappers handling their own error/fallback semantics
    (anyhow vs LashError vs DbError vs return-self-on-miss). Side benefit:
    lash-db's variant used to ignore `index.lash.md` — now it doesn't.

### Launch polish (Flawd dogfooding, 2026-08-08)

Found while running Flawd v0.7.0 against this repo. The `lash add` item is a
silent data-loss bug in our own write path and blocks the OSS launch; the other
two are rot in this repo's Flawd config, which means a contributor following
the README cannot run Flawd on Lash at all. Counterpart Flawd-side findings are
filed in the flawd repo under `tasks/tasks.fail-fast-degradation.md`.

- [x] `lash add` splices into a preceding multi-line note and silently destroys content #cli #bug #launch-blocker
  - LAUNCH BLOCKER — silent data loss in the primary write path. When the last
    task in the target file carries a multi-line `@agent-note`, `lash add`
    inserts the new task after the *first line* of that note instead of after
    the whole task block. On reindex the orphaned continuation lines are
    destroyed, not merely misattributed. Exit code is 0 with a normal success
    message, so nothing signals the loss
  - Minimal repro: a file with one task whose note is a `@agent-note:` line plus
    two continuation lines; run `lash add "second task" -f tasks/t.md -l x`. The
    new task lands between note line one and line two. After `lash index`,
    `lash show` on the first task returns only line one, and the second task has
    no note at all — lines two and three are gone from the index entirely
  - Hit twice unprompted while filing real tickets in the flawd repo, so this is
    not a contrived edge case: any repo using multi-line notes loses content on
    every subsequent add
  - Fix: insertion point must be the end of the full task block (checkbox line
    plus all indented continuation lines including `@`-directives). Regression
    tests: add after a task with (a) a multi-line note, (b) several
    `@`-directives, (c) a note containing a blank continuation line, (d) no note
    — assert byte-exact preservation of every pre-existing task and note, and
    that the new task lands last. Plus a round-trip property test: parse → add →
    parse preserves all prior notes verbatim
- [x] Remove the `[llm]` section from `flawd.toml` (removed in Flawd v0.7.0) #flawd #config
  - Our committed `flawd.toml` still carries an `[llm]` table (`mode = "off"`,
    `max_edit_chars = 120`). Flawd removed LLM semantic operators in v0.7.0 and
    treats a lingering `[llm]` table as a hard config error, so `flawd run` on a
    fresh clone dead-ends immediately and produces no output. The config had not
    been exercised since the last run in March, so this rotted unnoticed
  - Deleting those three lines is the whole fix — with `[llm]` gone the rest
    validates clean against v0.7.0 (`flawd config show`, no other stale keys)
  - Consider a CI job running `flawd config show` against the committed config
    so version drift in our own tool is caught by the build, not by a human
    months later
  - Done in #35. The CI job is not; it needs flawd installed on the runner,
    which is a bigger change than the fix
- [x] Make `flawd.toml`'s coverage command work in container mode #flawd #docker #ci
  - `flawd.toml` sets coverage to `cargo llvm-cov ...`, but the Dockerfile never
    installs `cargo-llvm-cov`. Flawd defaults to docker isolation, so the
    documented `flawd run` builds the image, runs coverage inside it, and fails
    with cargo exit 101 having written nothing
  - The committed command also hardcodes host paths (`LLVM_COV=/opt/homebrew/...`,
    `LLVM_PROFDATA=/opt/homebrew/...`) — macOS Homebrew paths that cannot resolve
    inside a Linux container
  - Fix: install `cargo-llvm-cov` plus `llvm-tools-preview` in the Dockerfile, or
    point `[coverage] command` at a tool the image has; make the paths portable
    rather than machine-specific. Verify by running `flawd run` with default
    isolation and confirming per-test collection succeeds instead of falling back
  - Done in #35. The Dockerfile installs `cargo-llvm-cov` from its prebuilt
    release (arch picked by `uname -m`) plus `llvm-tools-preview`, which
    supplies llvm-cov and llvm-profdata, so the command needs no path overrides
    at all. The command also creates `coverage/` first, since it is gitignored
    and so never exists in a fresh container. Verified with `flawd run` under
    default isolation: per-test targeting 6/6, no full-suite fallback
- [x] `@depends-on` comma form over-counts lines, appending outside the tasks section #cli #bug
  - Follow-up to the multi-line note fix, same function
    (`PlacementResolver::count_annotation_lines`). `count += depends_on.len()`
    assumes one source line per reference, which holds for what
    `MarkdownEmitter::format_task_annotations` writes. But the parser splits a
    hand-written comma-separated `@depends-on: a, b, c` into three references
    from a *single* line, so the count runs two lines long
  - Over-counting is not content-destroying like the note bug, but it is not
    harmless either: the insertion point escapes the task block. Repro — a file
    whose last task carries `@depends-on: a, b, c` on line 9, followed by a
    blank line, `## Notes` on line 11, then prose. `lash add "second task"`
    computes line 12 and writes the new task *after the `## Notes` heading*,
    landing it outside `## Tasks` entirely and splitting Notes from its prose.
    Correct placement is line 10
  - When the task is last in the file the emitter's `.min(lines.len())` clamp
    hides it — placement reported line 12 for a 10-line file and appended at the
    end, correct by accident. So this only bites when content follows
  - Fix needs the same information `@labels` lacks: whether the source used the
    inline or one-per-line form. Options: have the parser record the source line
    span for each annotation (fixes this, `@labels`, and any future variant at
    once), or count `min(depends_on.len(), actual_lines)` by re-reading the
    source. The first is the real fix; `TaskFile` currently retains no raw
    content, so it needs plumbing
  - Regression test: append after a task with comma-separated `@depends-on`
    followed by a section heading; assert the new task lands inside `## Tasks`
  - Fixed the way the ticket called for: the parser now records how many source
    lines a task's annotation block occupies, on `Task::annotation_line_count`,
    and the resolver uses that instead of re-deriving a count from the parsed
    metadata. This settles `@labels` too, which was previously uncountable for
    the same reason, and any future annotation that can be written in more than
    one shape. Tasks built in memory rather than parsed (the TUI, test
    fixtures) have no source lines to have counted, so they keep the derived
    estimate
- [x] `lash add --agent-note` with an embedded newline writes a malformed file and loses content #cli #bug #data-loss
  - Latent counterpart to the multi-line note fix, on the write side.
    `MarkdownEmitter::format_task_annotations` builds the note with
    `format!("{annotation_indent}@agent-note: {note}")` and pushes it as one
    "line". If the value contains a newline, the continuation is emitted with no
    indentation, so the parser stops at it and the content is dropped
  - I previously assessed this as unreachable from the CLI because notes are
    single-line. That is wrong — it is trivially reachable:
    `lash add "noted task" -f tasks/t.md --agent-note "$(printf 'first\nsecond')"`
    writes `  @agent-note: first` followed by a bare unindented `second`, and
    `lash show` afterwards returns only "first". The second line is silently
    gone, exit code 0
  - Fix: re-indent continuation lines to the annotation indent when emitting, so
    a multi-line value round-trips through parse -> emit -> parse. Reject or
    escape values that cannot round-trip rather than writing a file the parser
    will silently truncate
  - Regression test: emit a task with a two-line note, reparse the written file,
    assert both lines survive; plus a round-trip property test over notes with
    varying line counts and indentation
  - Fixed on both halves. The emitter now writes each continuation line with
    the annotation indent, which is the shape the parser folds back into one
    value. Values that cannot round-trip regardless of indentation are
    rejected up front by `MarkdownEmitter::check_agent_note`: a blank
    continuation line (the parser skips blanks) or one starting with `@` (read
    back as a separate annotation). Both produce
    `E_CREATE_INVALID_AGENT_NOTE` and leave the file untouched. Leading
    whitespace on a continuation is normalized rather than rejected, since the
    parser trims it and no text is lost
- [x] `lash add` prepends above the H1 when the target file has no tasks yet, and the task is never indexed #cli #bug #data-loss
  - Severe: the task is written to disk, invisible to Lash, and `lash lint`
    passes clean. Found 2026-08-08 while verifying the multi-line note fix
  - Repro: a well-formed file with a header and an empty `## Tasks` section
    (`# T` / `@id: t` / `## Tasks`). `lash add "Ship it" -f tasks/t.md -l docs`
    writes the checkbox at **line 1, above the `# T` heading**, reports
    `at tasks/t.md:0`, and a following `lash index` reports `0 task(s)`
  - Contrast confirming the trigger is the empty section: the identical command
    against the same file with one seed task appends correctly at line 9
  - Cause: `PlacementResolver::resolve_append` returns `line_number: 0` when
    `ctx.resolved_file.tasks.is_empty()`, commented "Signal for new file". The
    emitter maps `line_number == 0` to `insert_idx = 0`
    (`MarkdownEmitter::insert_into_existing`) and prepends. The sentinel
    conflates "brand-new file" with "existing file whose Tasks section is
    empty" — only the first is safe to write at offset 0
  - Fix: distinguish the two cases. For an existing file with no tasks, append
    at the end of the `## Tasks` section (`find_end_of_tasks_section` already
    exists and is used for the no-siblings-no-parent branch). Consider replacing
    the magic 0 with an explicit enum so the ambiguity cannot recur
  - Regression tests: add to (a) an existing file with an empty `## Tasks`
    section, (b) an existing file with a `## Tasks` section followed by other
    sections, (c) a genuinely new file — assert the header survives at line 1
    in (a) and (b), and that `lash index` finds the task in all three
  - Fixed by replacing the magic 0 with `InsertAnchor`, which distinguishes a
    concrete `Line(n)` from `EndOfTasksSection`. The emitter resolves the
    latter against the source text, since a parsed `TaskFile` records task
    line numbers but no section boundaries. New
    `parser::header::tasks_section_body` returns the section's line span and
    is markdown-aware, so a `##` inside a code fence does not close it. The
    reported line now comes from where the task was actually written, so the
    `:0` in the success message is gone too, as is the missing trailing
    newline noted under the ID ticket below
- [x] `lash format` deletes every section after `## Tasks` #cli #bug #data-loss #launch-blocker
  - Worse than the `lash add` bugs it was found next to, because `format` is
    documented as a normalizer and the README tells people to run it. Found
    2026-08-09 while checking that the empty-Tasks-section fix left files
    lint-clean
  - Repro: a file with `## Tasks`, then `## Notes` with prose, then
    `## References` with a link. `lash format` on it emits the header and the
    Tasks section and nothing else. Notes, References and all their content are
    gone. Exit code 0, no warning
  - This repo's own `lash.index.md` has a `## Description` section that sits
    *before* `## Tasks` and so survives, which is probably why nobody hit it
  - Cause is in the emit half of the formatter: it reconstructs the file from
    the parsed model, and the model keeps the header, the description and the
    task tree but nothing about sections it does not understand. Anything the
    parser did not model is not written back
  - Fix: the formatter must preserve source it does not model. Either carry the
    unmodelled tail through the parse (a raw span per unrecognized section) or
    have the formatter rewrite in place, editing only the lines it owns rather
    than regenerating the file
  - Regression tests: format a file with sections before and after `## Tasks`
    and assert byte-exact preservation of both; a file with a trailing section
    containing a fenced code block; and a round-trip property test asserting
    `format(format(x)) == format(x)`
  - Turned out to be broader than filed: sections *before* `## Tasks` were
    destroyed too (a `## Background` between the header and the tasks), because
    the parser folds them into "overview" text that `TaskFile` never stored.
    Fixed by taking the source alongside the parsed file: `format_file` now
    regenerates only the spans it owns (the H1 plus annotation block, the
    Description section, the Tasks section) and copies every other line
    through unchanged
- [x] `lash format` duplicates inline labels on every run #cli #bug
  - Not idempotent, and the file grows without bound. `- [ ] task one #docs
    #infra` becomes `#docs #infra #docs #infra` after one run and
    `#docs #infra #docs #infra #docs #infra` after two
  - The parser records inline labels in `metadata.labels` while leaving them in
    the title text, so the emitter writes the title (labels included) and then
    appends the labels again. Same double-counting shape as the annotation-line
    bugs: one piece of information recorded in two places with no note of which
    is authoritative
  - `lash format --check` reports a file as needing formatting forever, since
    the output never reaches a fixed point
  - Fix: strip inline labels from the stored title, or have the emitter write
    the title verbatim and skip the label pass. Whichever, add the
    `format(format(x)) == format(x)` property test above; it catches this and
    the section-deletion bug at once
  - Fixed on the emit side: the formatter strips inline labels from the title
    and writes the sorted metadata list as the single source. Leaving them in
    the parsed title keeps `lash list` and search working the way they do
    today. The idempotence property test covers both bugs, as predicted
- [x] `lash add` reports a task ID that does not match the indexed one #cli #bug #ux
  - The ID printed on creation cannot be used with `lash show` / `lash complete`
    / `@depends-on`, so any workflow that copies it fails. Observed repeatedly
    while filing tickets on 2026-08-08
  - Repro: `lash add "Ship v0.7.0 release notes" -f tasks/t.md -l docs` prints
    `Created task ship-v0-7-0-release-notes`, but the indexed id is
    `t#ship-v070-release-notes-docs`. Three separate discrepancies in one line:
    1. Version numbers slug differently — `v0-7-0` when reported, `v070` when
       indexed. Same input, two normalizations
    2. The label tag is folded into the identity — `#docs` becomes a `-docs`
       suffix. A task's id should not change because someone added a label
    3. Long titles are truncated to 40 chars on index but reported in full
       (e.g. reported `coverage-fail-fast-when-the-coverage-command-fails-and-its-output-predates-the-run`,
       indexed `coverage-fail-fast-when-the-coverage-com`)
  - Consequence beyond ergonomics: `@depends-on` written against the reported id
    dangles, and #27's dangling-reference check will reject it
  - Fix: derive the id once, in one place, and have `lash add` report exactly
    what the indexer will store. Label folding looks like the slug being
    computed from the raw checkbox line rather than the parsed title — labels
    should be excluded from identity
  - Minor, same code path: the appended final line has no trailing newline, so
    the file ends mid-line and diffs show "\ No newline at end of file"
  - Fixed by deriving the id in one place, `lash_types::task::synthesize_task_id`,
    which both the parser and `lash add` now call. Inline labels are stripped
    before slugging, so adding a label no longer changes a task's identity, and
    the 40-char truncation applies to both. On top of that the creation service
    re-reads the file it just wrote and reports the id the parser assigned,
    which is the only way to get the numeric suffix right when a synthesized id
    collides
  - Behaviour change worth noting on upgrade: synthesized ids for existing
    tasks change where a title contained an inline label or punctuation inside
    a word (`implement-authentication-security` becomes
    `implement-authentication`, `design-requestresponse-schemas` becomes
    `design-request-response-schemas`). A `@depends-on` written against an old
    synthesized id needs updating; explicit `@id:` values are untouched
  - The trailing-newline half was fixed with the empty-Tasks-section ticket
    above, since it lives in the same function
- [x] `test_user_config_save_and_load` writes to the real `~/.lash/config.toml` and can leave the CLI broken #testing #bug #isolation
  - A unit test in `crates/lash-types/src/config.rs` (`test_user_config_save_and_land`
    at ~L727) constructs a `UserConfig` with `color_scheme = "Test Theme"` and
    calls `config.save()`, which writes the developer's REAL home-directory
    config. It restores at the end via `UserConfig::default().save()`
  - Two problems with that. Even on success it does not restore what the user
    had — it overwrites with defaults, silently destroying real settings. And if
    the test panics, is filtered out mid-run, or the process is killed, the
    restore never executes and `~/.lash/config.toml` is left containing
    `color_scheme = "Test Theme"`, which no theme resolves. Every subsequent
    `lash` invocation then fails with
    `error: Color scheme 'Test Theme' not found`
  - Observed 2026-08-09: after a Flawd mutation run executed the suite hundreds
    of times (with mutants killed mid-execution), `~/.lash/config.toml` was left
    with the test value, mtime inside the run window. `lash index` broke
    machine-wide until the line was removed by hand. Mutation testing did not
    cause the bug, it amplified it — any interrupted `cargo test` does the same
  - The neighbouring `test_user_config_load_nonexistent` comments "This test
    assumes ~/.lash/config.toml doesn't exist or is valid", which acknowledges
    the hazard without containing it
  - Fix: never touch the real home directory from tests. Parameterise the config
    path (e.g. `UserConfig::save_to(path)` / `load_from(path)`) and point the
    test at a `tempfile::TempDir`, or gate resolution behind an env var the test
    sets. Restoring in a `Drop` guard is not sufficient on its own — a SIGKILL
    still skips it, and it still cannot restore settings it never captured
  - Audit the rest of the suite for the same pattern: `crates/lash-cli/tests/`
    has several files referencing `user_config_path`
  - Done in #34. `UserConfig::load_from(path)` / `save_to(path)` take the path
    explicitly and the tests point at a `TempDir`; `load()` and `save()` are
    thin wrappers, so production behaviour is unchanged. Verified by running
    the full suite and confirming the real `~/.lash/config.toml` came out
    byte-identical
  - Audit found nothing else. lash-cli's `Config::user_config_path` resolves a
    different location (`dirs::config_dir`) and its test only inspects the
    path; lash-tui's `apply_selected_theme` calls `save()` for real, which is
    correct, and no test exercises it
