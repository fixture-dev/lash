# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the major version is 0, minor version bumps may contain breaking changes.

## [Unreleased]

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

[Unreleased]: https://github.com/fixture-dev/lash/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/fixture-dev/lash/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/fixture-dev/lash/releases/tag/v0.1.0
