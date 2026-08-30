# CLI Task Mutation & Display Gaps (GitHub issues #23–#27)

Fixes for the task-mutation and display gaps reported while dogfooding
(fixture-site) ahead of open-sourcing. All five are CLI-layer issues.

## Tasks

- [x] Fix `lash add --id` silently discarded (#24)
  - [x] Write `@id: <slug>` annotation under the created task
  - [x] Success message and index reflect the explicit ID
  - [x] Reject invalid slugs loudly
  - [x] Tests: annotation written, task resolvable via `show <file>#<id>`
- [x] Validate `lash add --depends-on` refs at add time (#27)
  - [x] Unresolvable ref → hard error by default (no file written)
  - [x] `--allow-forward-ref` downgrades to warning for create-in-any-order
  - [x] Tests: dangling ref rejected, forward-ref flag allows with warning
- [x] Add `lash waive` command (#23)
  - [x] Mirrors `complete`: fuzzy resolver, `--dry-run`, `--cascade`, JSON
  - [x] Writes `- [-]` marker and reindexes atomically
  - [x] Optional `--reason "<text>"` records rationale
  - [x] Tests: status transition, cascade, dry-run, already-waived error
- [x] `lash show` displays full task record (#26)
  - [x] `@agent-note`, `@depends-on` (with each dependency's current status)
  - [x] One-line-per-child summary with checkbox states
  - [x] `--short` flag preserves current terse output
  - [x] Tests: full output fields, dependency status, `--short`
- [x] Add `lash update` command (#25)
  - [x] `--title` with ID stability (auto-pin `@id:` for derived-slug tasks)
  - [x] `--add-label` / `--remove-label`
  - [x] `--estimate`, `--owner`
  - [x] `--agent-note` (replace) / `--append-agent-note`
  - [x] `--add-depends-on` / `--remove-depends-on`, validated against index
  - [x] Tests: each field, ID stability on retitle, dangling-dep rejection
- [x] Fix `--append-agent-note` duplicating `@agent-note` when body text precedes the note (#74)
  - [x] Annotation lookup scans the task's whole body (mirrors the parser's orphaned-annotation merge)
  - [x] Tests: append/replace past body text, no reach into next task, lint-clean round trip
