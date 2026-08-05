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
- [ ] Add `lash waive` command (#23)
  - [ ] Mirrors `complete`: fuzzy resolver, `--dry-run`, `--cascade`, JSON
  - [ ] Writes `- [-]` marker and reindexes atomically
  - [ ] Optional `--reason "<text>"` records rationale
  - [ ] Tests: status transition, cascade, dry-run, already-waived error
- [ ] `lash show` displays full task record (#26)
  - [ ] `@agent-note`, `@depends-on` (with each dependency's current status)
  - [ ] One-line-per-child summary with checkbox states
  - [ ] `--short` flag preserves current terse output
  - [ ] Tests: full output fields, dependency status, `--short`
- [ ] Add `lash update` command (#25)
  - [ ] `--title` with ID stability (auto-pin `@id:` for derived-slug tasks)
  - [ ] `--add-label` / `--remove-label`
  - [ ] `--estimate`, `--owner`
  - [ ] `--agent-note` (replace) / `--append-agent-note`
  - [ ] `--add-depends-on` / `--remove-depends-on`, validated against index
  - [ ] Tests: each field, ID stability on retitle, dangling-dep rejection
