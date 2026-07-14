# Dependency & ID Resolution Fixes (GitHub issues #14–#19)

Fixes for the cluster of `@depends-on` / `@id` resolution bugs reported in
GitHub issues #14–#19. Root cause: three divergent resolution paths (linter
rule, unused graph resolver, DB full-id lookup) and a resolver that only
understood the undocumented `file-id#fragment-slug` form.

## Tasks

- [ ] Shared reference resolver (`lash-core/src/dependency/reference.rs`)
  - [ ] Canonical resolution: bare `@id`, `#task:id`, `#id`, `file-id#task:id`,
        `file-id#id`, `file.md`, `file.md#task:id`, bare file-id (file-level)
  - [ ] Unit tests for every form
- [ ] #16 Comma-separated `@depends-on` split into multiple refs
- [ ] #15 Linter rule resolves all documented + natural forms via shared resolver
- [ ] #18 `E_LINK_NOT_FOUND` reports the `@depends-on:` line, not `:0:0`
- [ ] #19 `check-links` validates `@depends-on` via the same linter rule
- [ ] #14 `show`/`start`/`complete` resolve bare `@id` targets; fix `show` E_INTERNAL→E_NOT_FOUND
- [ ] #17 `complete` refuses when a resolvable dependency is unmet (with `--force`)
- [ ] Verify all six issue repros; update devlog
