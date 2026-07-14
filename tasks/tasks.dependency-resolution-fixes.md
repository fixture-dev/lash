# Dependency & ID Resolution Fixes (GitHub issues #14–#19)

Fixes for the cluster of `@depends-on` / `@id` resolution bugs reported in
GitHub issues #14–#19. Root cause: three divergent resolution paths (linter
rule, unused graph resolver, DB full-id lookup) and a resolver that only
understood the undocumented `file-id#fragment-slug` form.

## Tasks

- [x] Shared reference resolver (`lash-core/src/dependency/reference.rs`)
  - [x] Canonical resolution: bare `@id`, `#task:id`, `#id`, `file-id#task:id`,
        `file-id#id`, `file.md`, `file.md#task:id`, bare file-id (file-level)
  - [x] Unit tests for every form
- [x] #16 Comma-separated `@depends-on` split into multiple refs
- [x] #15 Linter rule resolves all documented + natural forms via shared resolver
- [x] #18 `E_LINK_NOT_FOUND` reports the `@depends-on:` line, not `:0:0`
- [x] #19 `check-links` validates `@depends-on` via the same shared resolver
- [x] #14 `show`/`start`/`complete` resolve bare `@id` targets; fix `show` E_INTERNAL→not-found
- [x] #17 `complete` refuses when a resolvable dependency is unmet (with `--force`)
- [x] Verify all six issue repros; skill docs (`dependencies.md`) updated
