# Unit Test Coverage Gap Analysis

**Date:** 2025-11-23
**Overall Coverage:** 73.32% (5385/7345 lines covered)
**Target:** >80% overall, >90% for critical modules

---

## Executive Summary

The Lash project currently has **896 unit tests** distributed across 6 crates with **73.32% overall coverage**. This analysis identifies specific gaps in unit test coverage according to Task 2 requirements in `tasks/tasks.testing.md`.

### Critical Findings

1. **Below Target:** Overall coverage is 6.68% below the 80% target
2. **TUI Module:** 0% coverage (0/380 lines) - completely untested
3. **Search Module:** Only 24.8% coverage (67/270 lines) - critical functionality
4. **Error Handling:** 62.8% coverage (167/266 lines) - needs improvement
5. **Database Repository:** Multiple modules at 60-80% coverage

### Coverage by Crate

| Crate | Tests | Coverage | Status |
|-------|-------|----------|--------|
| lash-types | 138 | ~75% | Needs improvement |
| lash-core | 501 | ~85% | Good (critical module) |
| lash-db | 136 | ~65% | Below target |
| lash-cli | 81 | ~60% | Below target |
| lash-agent | 40 | ~88% | Good |
| lash-tui | 0 | 0% | Not started |

---

## Detailed Gap Analysis by Module Category

### 1. Core Data Model Tests (lash-types)

#### Current Coverage
- **status.rs:** 50/52 lines (96%) - ✅ EXCELLENT
- **task.rs:** 119/133 lines (89%) - ✅ GOOD
- **dependency.rs:** 84/92 lines (91%) - ✅ GOOD
- **label.rs:** 51/51 lines (100%) - ✅ EXCELLENT
- **file.rs:** 38/41 lines (93%) - ✅ GOOD

#### Missing Tests
- **task.rs (14 uncovered lines):**
  - Lines 84-89: Task validation logic edge cases
  - Lines 106, 108-109: Additional validation paths
  - Lines 121-122: Builder pattern edge cases
  - Lines 270-272: TaskTree manipulation methods

- **dependency.rs (8 uncovered lines):**
  - Dependency parsing error paths
  - Edge cases in dependency reference resolution

**Priority:** MEDIUM
**Effort:** 2-4 hours
**Impact:** Low risk - already well tested

---

### 2. Markdown Parser Tests (lash-core/parser)

#### Current Coverage
- **parser/mod.rs:** 90/155 lines (58%) - ⚠️ CRITICAL GAP
- **parser/annotations.rs:** 176/182 lines (97%) - ✅ EXCELLENT
- **parser/builder.rs:** 97/100 lines (97%) - ✅ EXCELLENT
- **parser/checkbox.rs:** 75/76 lines (99%) - ✅ EXCELLENT
- **parser/header.rs:** 139/149 lines (93%) - ✅ GOOD
- **parser/events.rs:** 20/34 lines (59%) - ⚠️ NEEDS WORK

#### Missing Tests
- **parser/mod.rs (65 uncovered lines):**
  - ParseContext construction and manipulation
  - Error aggregation in parse_file()
  - File I/O error handling
  - Hash computation edge cases
  - Section tracking during parsing

- **parser/events.rs (14 uncovered lines):**
  - Event stream processing edge cases
  - Line number tracking corner cases
  - Markdown event handling for uncommon elements

**Priority:** CRITICAL
**Effort:** 1 day
**Impact:** High risk - core parsing logic

**Specific Test Needs:**
```rust
// parser/mod.rs
- test_parse_file_io_error()
- test_parse_file_empty()
- test_parse_context_section_tracking()
- test_error_aggregation_multiple_errors()
- test_parse_file_hash_computation()

// parser/events.rs
- test_event_stream_line_tracking()
- test_event_stream_nested_blocks()
- test_event_stream_malformed_markdown()
```

---

### 3. Linter Tests (lash-core/linter)

#### Current Coverage
- **linter/linter.rs:** 46/49 lines (94%) - ✅ GOOD
- **linter/registry.rs:** 90/92 lines (98%) - ✅ EXCELLENT
- **linter/diagnostic.rs:** 40/66 lines (61%) - ⚠️ NEEDS WORK
- **linter/fix.rs:** 37/37 lines (100%) - ✅ EXCELLENT

#### Individual Linter Rules Coverage
Most linter rules have 75-95% coverage, but several need improvement:

- **rules/semantic/duplicate_id.rs:** 20/33 lines (61%)
- **rules/semantic/valid_date.rs:** 81/93 lines (87%)
- **rules/crossfile/dependency_exists.rs:** 83/130 lines (64%)
- **rules/crossfile/orphaned_files.rs:** 52/60 lines (87%)
- **rules/crossfile/valid_path_resolution.rs:** 77/86 lines (90%)

#### Missing Tests
- **linter/diagnostic.rs (26 uncovered lines):**
  - Diagnostic formatting with various contexts
  - JSON serialization edge cases
  - Snippet extraction and highlighting
  - Multi-line diagnostic formatting

- **rules/semantic/duplicate_id.rs:**
  - Edge cases in duplicate detection
  - Performance with many IDs
  - Error message formatting

- **rules/crossfile/dependency_exists.rs (47 uncovered lines):**
  - Cross-file dependency validation edge cases
  - Broken link detection scenarios
  - Error recovery and reporting

**Priority:** HIGH
**Effort:** 1-2 days
**Impact:** Medium risk - affects validation quality

**Specific Test Needs:**
```rust
// linter/diagnostic.rs
- test_diagnostic_format_multiline()
- test_diagnostic_json_serialization()
- test_diagnostic_snippet_extraction()
- test_diagnostic_with_fixes()

// rules/semantic/duplicate_id.rs
- test_duplicate_id_across_sections()
- test_duplicate_id_case_sensitivity()
- test_duplicate_id_error_message()

// rules/crossfile/dependency_exists.rs
- test_cross_file_dependency_missing_file()
- test_cross_file_dependency_missing_task()
- test_cross_file_dependency_relative_paths()
```

---

### 4. Dependency Resolution Tests (lash-core/dependency)

#### Current Coverage
- **dependency/graph.rs:** 280/298 lines (94%) - ✅ EXCELLENT
- **dependency/blocker_analyzer.rs:** 186/197 lines (94%) - ✅ EXCELLENT
- **dependency/cycle_detector.rs:** 149/171 lines (87%) - ✅ GOOD
- **dependency/graph_exporter.rs:** 137/148 lines (93%) - ✅ GOOD
- **dependency/resolver.rs:** 115/191 lines (60%) - ⚠️ CRITICAL GAP
- **dependency/status_computer.rs:** 92/114 lines (81%) - ⚠️ NEEDS WORK

#### Missing Tests
- **dependency/resolver.rs (76 uncovered lines):**
  - Dependency resolution across multiple files
  - Complex dependency chains
  - Error handling for unresolvable dependencies
  - Directory-level dependency resolution
  - Performance with large graphs

- **dependency/status_computer.rs (22 uncovered lines):**
  - Status computation with blocked dependencies
  - Status propagation through chains
  - Edge cases in completion checking

**Priority:** CRITICAL
**Effort:** 1-2 days
**Impact:** High risk - core functionality

**Specific Test Needs:**
```rust
// dependency/resolver.rs
- test_resolve_cross_file_dependencies()
- test_resolve_directory_dependencies()
- test_resolve_unresolvable_dependency()
- test_resolve_circular_dependency_error()
- test_resolve_performance_large_graph()

// dependency/status_computer.rs
- test_compute_status_with_blockers()
- test_compute_status_propagation()
- test_compute_status_waived_dependencies()
- test_compute_status_mixed_states()
```

---

### 5. Database Tests (lash-db)

#### Current Coverage
- **connection.rs:** 33/34 lines (97%) - ✅ EXCELLENT
- **graph_builder.rs:** 34/34 lines (100%) - ✅ EXCELLENT
- **diff.rs:** 38/40 lines (95%) - ✅ EXCELLENT
- **repository/files.rs:** 143/182 lines (79%) - ⚠️ NEEDS WORK
- **repository/tasks.rs:** 120/183 lines (66%) - ⚠️ CRITICAL GAP
- **repository/labels.rs:** 81/93 lines (87%) - ⚠️ NEEDS WORK
- **repository/dependencies.rs:** 44/55 lines (80%) - ⚠️ NEEDS WORK
- **migrations.rs:** 8/30 lines (27%) - ⚠️ CRITICAL GAP
- **error.rs:** 0/4 lines (0%) - ⚠️ MISSING

#### Missing Tests
- **repository/tasks.rs (63 uncovered lines):**
  - Complex query operations
  - Bulk insert/update operations
  - Transaction rollback scenarios
  - Index integrity after updates
  - Filter combinations

- **repository/files.rs (39 uncovered lines):**
  - File CRUD edge cases
  - Hash-based change detection
  - Bulk operations
  - Error handling

- **migrations.rs (22 uncovered lines):**
  - Migration version tracking
  - Migration rollback (if supported)
  - Schema upgrade paths
  - Migration error recovery

- **error.rs (4 uncovered lines):**
  - Error construction and formatting
  - DbError variants

**Priority:** HIGH
**Effort:** 2-3 days
**Impact:** High risk - data integrity

**Specific Test Needs:**
```rust
// repository/tasks.rs
- test_query_tasks_complex_filters()
- test_bulk_insert_tasks()
- test_update_task_with_dependencies()
- test_transaction_rollback()
- test_query_performance_large_dataset()

// repository/files.rs
- test_file_crud_operations()
- test_file_hash_change_detection()
- test_bulk_file_operations()

// migrations.rs
- test_migration_version_tracking()
- test_migration_upgrade_path()
- test_migration_idempotency()
- test_migration_error_handling()

// error.rs
- test_db_error_variants()
- test_db_error_display()
- test_db_error_from_rusqlite()
```

---

### 6. Indexing Tests (lash-db)

#### Current Coverage
- **indexer.rs:** 160/186 lines (86%) - ✅ GOOD
- **walker.rs:** 86/113 lines (76%) - ⚠️ NEEDS WORK
- **verifier.rs:** 114/126 lines (90%) - ✅ GOOD
- **dependency_updater.rs:** 57/70 lines (81%) - ⚠️ NEEDS WORK
- **profiler.rs:** 84/125 lines (67%) - ⚠️ NEEDS WORK

#### Missing Tests
- **indexer.rs (26 uncovered lines):**
  - Incremental indexing edge cases
  - Error recovery during indexing
  - Progress reporting
  - Parallel indexing coordination

- **walker.rs (27 uncovered lines):**
  - File discovery with symlinks
  - Ignore patterns
  - Large directory traversal
  - Hidden files handling

- **dependency_updater.rs (13 uncovered lines):**
  - Dependency update propagation
  - Batch update operations
  - Error handling in updates

- **profiler.rs (41 uncovered lines):**
  - Profiling data collection
  - Report generation
  - Timing accuracy

**Priority:** MEDIUM
**Effort:** 1-2 days
**Impact:** Medium risk - performance and reliability

**Specific Test Needs:**
```rust
// indexer.rs
- test_incremental_indexing_edge_cases()
- test_indexing_error_recovery()
- test_progress_reporting_accuracy()

// walker.rs
- test_file_walker_symlinks()
- test_file_walker_ignore_patterns()
- test_file_walker_hidden_files()

// dependency_updater.rs
- test_dependency_update_propagation()
- test_batch_dependency_updates()

// profiler.rs
- test_profiler_data_collection()
- test_profiler_report_generation()
```

---

### 7. Search Tests (lash-db/search)

#### Current Coverage
- **search.rs:** 67/270 lines (24.8%) - 🚨 CRITICAL GAP

#### Missing Tests
This is the largest gap in the codebase. **203 uncovered lines** representing core search functionality.

**Missing Test Categories:**
1. **Query Parsing (est. 40 lines uncovered):**
   - parse_query() function
   - Complex query syntax
   - Boolean operators
   - Phrase queries
   - Filter parsing

2. **FTS5 Integration (est. 80 lines uncovered):**
   - Full-text search execution
   - Relevance scoring
   - Snippet generation
   - Highlighting

3. **Result Ranking (est. 40 lines uncovered):**
   - SearchScorer algorithm
   - Boost factors (title, prefix, label)
   - Score normalization
   - Result ordering

4. **Filtering and Pagination (est. 30 lines uncovered):**
   - Label filters
   - Status filters
   - Path scoping
   - Limit/offset handling

5. **Performance and Metrics (est. 13 lines uncovered):**
   - Timing collection
   - Cache hit tracking
   - Result counting

**Priority:** CRITICAL
**Effort:** 2-3 days
**Impact:** Very high risk - core user-facing functionality

**Specific Test Needs:**
```rust
// search.rs - Query Parsing
- test_parse_query_simple()
- test_parse_query_phrase()
- test_parse_query_boolean_operators()
- test_parse_query_filters()
- test_parse_query_invalid_syntax()

// search.rs - FTS5 Integration
- test_search_fts5_basic()
- test_search_fts5_relevance()
- test_search_fts5_snippet_generation()
- test_search_fts5_highlighting()
- test_search_fts5_prefix_matching()

// search.rs - Ranking
- test_search_scorer_title_boost()
- test_search_scorer_prefix_boost()
- test_search_scorer_label_boost()
- test_search_scorer_combined_scoring()

// search.rs - Filtering
- test_search_filter_by_labels()
- test_search_filter_by_status()
- test_search_filter_by_path()
- test_search_filter_combined()

// search.rs - Pagination
- test_search_pagination_limit()
- test_search_pagination_offset()
- test_search_pagination_beyond_results()

// search.rs - Performance
- test_search_metrics_collection()
- test_search_with_profiling()
```

---

### 8. CLI Framework Tests (lash-cli)

#### Current Coverage
- **cli.rs:** Good coverage from unit tests
- **config.rs:** 70/77 lines (91%) - ✅ GOOD
- **formatter.rs:** 94/121 lines (78%) - ⚠️ NEEDS WORK
- **logging.rs:** 30/116 lines (26%) - 🚨 CRITICAL GAP
- **progress.rs:** 30/103 lines (29%) - 🚨 CRITICAL GAP
- **context.rs:** 43/45 lines (96%) - ✅ EXCELLENT
- **project_root.rs:** 40/45 lines (89%) - ✅ GOOD
- **command_utils.rs:** 10/24 lines (42%) - ⚠️ NEEDS WORK
- **commands/check_links/core.rs:** 0/26 lines (0%) - 🚨 MISSING

#### Missing Tests
- **logging.rs (86 uncovered lines):**
  - Logger initialization
  - Log level configuration
  - Panic hook installation
  - Diagnostic info collection
  - Log formatting

- **progress.rs (73 uncovered lines):**
  - Progress bar creation and updates
  - Multi-progress handling
  - Progress completion
  - Error display with progress

- **formatter.rs (27 uncovered lines):**
  - Complex output formatting
  - Color handling edge cases
  - JSON formatting variations

- **command_utils.rs (14 uncovered lines):**
  - Command utility functions
  - DB connection helpers
  - Parser initialization

- **commands/check_links/core.rs (26 uncovered lines):**
  - Link checking logic
  - Link resolution
  - Error reporting

**Priority:** MEDIUM
**Effort:** 1-2 days
**Impact:** Medium risk - affects UX but not correctness

**Specific Test Needs:**
```rust
// logging.rs
- test_logger_initialization()
- test_log_level_from_verbosity()
- test_panic_hook_installation()
- test_diagnostic_info_collection()

// progress.rs
- test_progress_bar_creation()
- test_progress_updates()
- test_multi_progress_coordination()
- test_progress_completion()

// formatter.rs
- test_format_task_list_complex()
- test_format_json_nested()
- test_format_with_color_codes()

// command_utils.rs
- test_ensure_indexed()
- test_get_db_connection()
- test_get_parser()

// commands/check_links/core.rs
- test_check_links_valid()
- test_check_links_broken()
- test_check_links_resolution()
```

---

### 9. Error Handling Tests (lash-types/error)

#### Current Coverage
- **error.rs:** 167/266 lines (62.8%) - ⚠️ NEEDS WORK

#### Missing Tests
**99 uncovered lines** covering:

1. **Error Construction (est. 30 lines):**
   - Convenience constructors for each error variant
   - Error with location info
   - Error with suggestions

2. **Error Formatting (est. 40 lines):**
   - Display implementation
   - Debug formatting
   - JSON serialization
   - Pretty printing with colors

3. **Diagnostic Conversion (est. 20 lines):**
   - to_diagnostic() method
   - Diagnostic aggregation
   - Severity mapping

4. **Exit Code Mapping (est. 9 lines):**
   - ExitCode::from() for various error types
   - Edge cases

**Priority:** HIGH
**Effort:** 1 day
**Impact:** Medium risk - affects error reporting quality

**Specific Test Needs:**
```rust
// error.rs
- test_error_construction_all_variants()
- test_error_with_location()
- test_error_with_suggestions()
- test_error_display_formatting()
- test_error_json_serialization()
- test_error_to_diagnostic()
- test_diagnostic_aggregation()
- test_exit_code_mapping()
- test_exit_code_conversion()
```

---

### 10. TUI Tests (lash-tui)

#### Current Coverage
- **All modules:** 0/380 lines (0%) - 🚨 NOT STARTED

This is a special case. TUI testing is inherently difficult and has lower priority.

**Missing Tests (Entire Module):**
- app.rs (84 lines)
- event.rs (26 lines)
- state.rs (37 lines)
- terminal.rs (15 lines)
- ui/* (185 lines)

**Priority:** LOW (per project guidelines: "Less critical: >70%")
**Effort:** 3-4 days
**Impact:** Low risk - TUI is supplementary to CLI

**Recommendation:** Defer TUI unit tests until core modules reach >90% coverage. Focus on integration tests for TUI instead.

---

## Priority-Ordered Implementation Plan

### Phase 1: Critical Gaps (Week 1)
**Target: Bring critical modules to >90%**

1. **Search Module** (2-3 days) - Currently 24.8%
   - 203 lines to cover
   - Most critical user-facing gap
   - Implement comprehensive FTS5 tests

2. **Parser Main Module** (1 day) - Currently 58%
   - 65 lines to cover
   - Core parsing logic
   - File I/O and error aggregation

3. **Dependency Resolver** (1-2 days) - Currently 60%
   - 76 lines to cover
   - Critical for correctness
   - Cross-file resolution tests

### Phase 2: High Priority Gaps (Week 2)
**Target: Bring all critical modules above 80%**

4. **Database Repository/Tasks** (2 days) - Currently 66%
   - 63 lines to cover
   - Data integrity critical
   - Complex query testing

5. **Error Handling** (1 day) - Currently 62.8%
   - 99 lines to cover
   - Affects all error reporting
   - Test all error variants

6. **Linter Diagnostic** (1 day) - Currently 61%
   - 26 lines to cover
   - Affects validation UX
   - Test formatting and serialization

7. **Cross-file Linter Rules** (1 day) - Various 60-90%
   - ~60 lines to cover across multiple rules
   - Important for validation

### Phase 3: Medium Priority Gaps (Week 3)
**Target: Bring overall coverage to >80%**

8. **CLI Logging/Progress** (1-2 days) - Currently 26-29%
   - 159 lines to cover
   - UX impact but not correctness
   - Test output and formatting

9. **Database Migrations** (0.5 day) - Currently 27%
   - 22 lines to cover
   - Critical but small scope
   - Test upgrade paths

10. **Indexing Components** (1 day) - Currently 76-86%
    - ~70 lines to cover
    - Walker, profiler, dependency updater
    - Edge case coverage

### Phase 4: Polish and Completion (Week 4)
**Target: Reach >85% overall coverage**

11. **Remaining Data Model Gaps** (0.5 day)
    - Task, dependency edge cases
    - ~25 lines to cover

12. **Parser Events and Formatting** (0.5 day)
    - Event stream edge cases
    - Formatter edge cases
    - ~40 lines to cover

13. **CLI Command Utilities** (0.5 day)
    - check_links core
    - command_utils
    - ~40 lines to cover

14. **Final Coverage Push** (1 day)
    - Address any remaining gaps
    - Focus on critical paths
    - Verify coverage in CI

---

## Files Requiring Immediate Attention

### Critical (Must address in Phase 1-2)
1. `crates/lash-db/src/search.rs` - 203 lines uncovered
2. `crates/lash-core/src/parser/mod.rs` - 65 lines uncovered
3. `crates/lash-types/src/error.rs` - 99 lines uncovered
4. `crates/lash-db/src/repository/tasks.rs` - 63 lines uncovered
5. `crates/lash-core/src/dependency/resolver.rs` - 76 lines uncovered
6. `crates/lash-cli/src/logging.rs` - 86 lines uncovered
7. `crates/lash-cli/src/progress.rs` - 73 lines uncovered

### High Priority (Phase 2-3)
8. `crates/lash-db/src/repository/files.rs` - 39 lines uncovered
9. `crates/lash-core/src/linter/diagnostic.rs` - 26 lines uncovered
10. `crates/lash-core/src/dependency/status_computer.rs` - 22 lines uncovered
11. `crates/lash-db/src/profiler.rs` - 41 lines uncovered
12. `crates/lash-db/src/migrations.rs` - 22 lines uncovered

---

## Coverage Targets by Module (from Task 2)

| Module Category | Target | Current | Gap | Status |
|----------------|--------|---------|-----|--------|
| Parser | >90% | ~85%* | ~5% | ⚠️ Close |
| Linter | >90% | ~90%* | ~0% | ✅ Met |
| Dependency | >90% | ~80%* | ~10% | ⚠️ Gap |
| Database | >80% | ~65%* | ~15% | 🚨 Below |
| Search | >80% | 24.8% | ~55% | 🚨 Critical |
| Error Handling | >80% | 62.8% | ~17% | 🚨 Below |
| CLI Framework | >80% | ~60%* | ~20% | 🚨 Below |
| Data Model | >80% | ~85%* | 0% | ✅ Exceeds |
| TUI | >70% | 0% | ~70% | 🚨 Missing |

\* Estimated weighted average across submodules

---

## Recommendations

### Immediate Actions (This Week)
1. **Focus on search.rs** - This is the single largest gap and critical for UX
2. **Fix parser/mod.rs** - Core parsing logic needs better coverage
3. **Address dependency/resolver.rs** - Critical for correctness

### Short-term (Next 2 Weeks)
4. Complete database repository tests (tasks, files)
5. Improve error handling coverage
6. Add missing linter rule tests
7. Cover CLI logging and progress modules

### Medium-term (Month 1)
8. Bring all critical modules to >90% coverage
9. Bring overall coverage to >85%
10. Defer TUI unit tests (use integration tests instead)

### Quality Guidelines
- **No frivolous tests** - Test behavior, not trivial getters/setters
- **No test-specific code in production** - Use proper test fixtures
- **Test at the right level** - Unit tests for logic, integration for workflows
- **Fast tests** - Keep unit tests under 100ms each
- **Deterministic tests** - No flaky tests allowed

### Coverage Enforcement
- Current CI coverage threshold: 80%
- Recommend gradual increase as gaps close:
  - After Phase 1: Set to 75%
  - After Phase 2: Set to 78%
  - After Phase 3: Set to 80%
  - After Phase 4: Set to 82%

---

## Test Implementation Examples

### Example 1: Search Module Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_phrase() {
        let query = r#""exact phrase" filter:label"#;
        let parsed = parse_query(query).unwrap();

        assert!(parsed.contains_phrase("exact phrase"));
        assert_eq!(parsed.filters.len(), 1);
        assert_eq!(parsed.filters[0].field, "label");
    }

    #[test]
    fn test_search_fts5_relevance_scoring() {
        let conn = create_test_db();
        insert_test_tasks(&conn);

        let results = search(&conn, "important task", &SearchQuery::default()).unwrap();

        // Title matches should rank higher than body matches
        assert!(results[0].title.contains("important"));
        assert!(results[0].score > results[1].score);
    }
}
```

### Example 2: Parser Error Aggregation Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_aggregates_multiple_errors() {
        let content = r#"
# Tasks
- [*] Invalid checkbox
- [ ] Valid task
- [?] Another invalid checkbox
        "#;

        let result = parse_file_from_string("test.md", content, &LashConfig::default());

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2); // Two invalid checkboxes
        assert!(errors[0].message.contains("Invalid checkbox character"));
        assert!(errors[1].message.contains("Invalid checkbox character"));
    }
}
```

### Example 3: Database Transaction Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_repository_transaction_rollback() {
        let conn = create_test_db();
        let repo = TaskRepository::new(&conn);

        let tx = conn.transaction().unwrap();
        repo.insert_task(&tx, &create_test_task()).unwrap();

        // Force an error condition
        let invalid_task = Task { id: "".to_string(), ..create_test_task() };
        assert!(repo.insert_task(&tx, &invalid_task).is_err());

        // Rollback should occur
        tx.rollback().unwrap();

        // Verify first insert was rolled back
        assert_eq!(repo.count_tasks(&conn).unwrap(), 0);
    }
}
```

---

## Summary

The Lash project has a solid testing foundation with 896 unit tests, but significant gaps remain:

- **3 Critical Gaps:** Search (24.8%), Logging (26%), Migrations (27%)
- **7 High Priority Gaps:** Parser main, resolver, error handling, DB tasks, progress, profiler, linter diagnostics
- **Overall:** Need to add approximately 1200-1500 lines of test code to reach 85% coverage

**Estimated effort to reach >85% overall coverage: 3-4 weeks**

The project is well-positioned to reach coverage targets with focused effort on the identified gaps, prioritizing critical modules (search, parser, dependency) before less critical modules (TUI, CLI utilities).
