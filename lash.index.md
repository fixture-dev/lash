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
- [ ] `formatter::format_file_in_place` uses `write_atomic` #core
- [ ] Stale-modal banner for in-flight modals on external change #tui
- [ ] Bounded watcher channel with `FullReload` overflow path #core

### CLI hygiene

- [x] Cap project-root search at the enclosing git repository #cli
- [ ] Consolidate the four `find_project_root` implementations into one #cli #refactor
  - Today lash-cli (×2), lash-db, and lash-types each have their own; they
    share a `find_git_root` helper now but the discovery loops are still
    duplicated
