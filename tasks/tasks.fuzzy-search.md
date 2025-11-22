# Fuzzy Search Tasks

**Module:** `lash-db` (search subsystem)
**Dependencies:** tasks.sqlite-schema.md, tasks.indexing.md
**Effort:** 4-6 days
**Priority:** HIGH

## Overview

Implement fuzzy search functionality to allow users and agents to quickly find tasks by partial matching on titles, descriptions, labels, and file names. The search must be fast, rank results by relevance, and handle typos gracefully.

## Core Requirements

From design-doc.md:
- Fuzzy search across titles, bodies, labels, filenames (section 7.3.3)
- Fast performance for interactive use (section 9.1)
- Support for two approaches: SQLite FTS or in-Rust fuzzy matcher (section 9.3)

---

## Task 1: Search Index Schema

**Priority:** CRITICAL
**Effort:** 1 day
**Depends on:** tasks.sqlite-schema.md#1

### Description

Design and implement the database schema for search indexing, likely using SQLite FTS5.

### Subtasks

- [x] Research FTS5 vs in-memory fuzzy matching
  - [x] Benchmark FTS5 query performance
  - [x] Evaluate fuzzy matching libraries (fuzzy-matcher, sublime_fuzzy)
  - [x] Consider hybrid approach: FTS5 + in-memory ranking
- [x] Define FTS5 virtual table (if using FTS5)
  - [x] `search_index` table with columns:
    - [x] `task_id` (FK to tasks.id)
    - [x] `content` (combined searchable text)
    - [x] `title` (task title, higher weight)
    - [x] `body` (task body, lower weight)
    - [x] `labels` (space-separated labels)
    - [x] `file_path` (for filename matching)
  - [x] Configure tokenizer (unicode61 or porter)
  - [x] Set up column weights (title > labels > body)
- [x] Implement search index population
  - [x] During indexing, populate FTS5 table
  - [x] Extract text from tasks: title + body + labels + path
  - [x] Insert into search_index
- [x] Add index maintenance
  - [x] Update search index when tasks change
  - [x] Delete from index when tasks deleted
  - [x] Rebuild index command

### Success Criteria

- Search index correctly represents all searchable content
- Index stays in sync with tasks table
- FTS5 queries are fast (<100ms for typical searches)

### Tests

- Unit: Test index population logic
- Integration: Populate index from fixture data
- Integration: Verify index updates when tasks change
- Performance: Benchmark index size and query speed

---

## Task 2: Fuzzy Match Scoring

**Priority:** HIGH
**Effort:** 1-2 days
**Depends on:** Task 1

### Description

Implement relevance scoring and ranking for search results.

### Subtasks

- [x] Define scoring algorithm
  - [x] Base score from FTS5 ranking (bm25 or similar)
  - [x] Boost for exact prefix matches
  - [x] Boost for title matches vs body matches
  - [x] Penalty for long documents (normalize)
- [x] Implement `SearchScorer` struct
  - [x] Take query and matched document
  - [x] Compute relevance score (0.0 to 1.0)
  - [x] Support multiple ranking strategies
- [x] Add term highlighting
  - [x] Identify matched terms in results
  - [x] Store match positions for highlighting
  - [x] Support context snippets (show surrounding text)
- [x] Implement result ranking
  - [x] Sort by score descending
  - [x] Break ties by created date or path
  - [x] Support pagination (offset + limit)
- [x] Tune scoring weights
  - [x] Experiment with different weight combinations
  - [x] Test against representative queries
  - [x] Document tuning rationale

### Success Criteria

- Results are ranked by relevance
- Most relevant results appear first
- Scoring handles edge cases (empty query, etc.)
- Highlighting is accurate

### Tests

- Unit: Test scoring algorithm with various inputs
- Unit: Test term highlighting logic
- Integration: Search for known terms, verify ranking
- Manual: Qualitative assessment of result quality

---

## Task 3: Search Query API

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1, Task 2

### Description

Implement the search query API that the `lash search` command will use.

### Subtasks

- [x] Define `SearchQuery` struct
  - [x] `query`: search string
  - [x] `scope`: optional path filter
  - [x] `limit`: max results (default 20)
  - [x] `offset`: pagination offset (default 0)
  - [x] `filters`: label, status, etc.
- [x] Implement `search()` function
  - [x] Parse query string
  - [x] Build FTS5 query (or run fuzzy matcher)
  - [x] Apply scope and filters
  - [x] Execute search
  - [x] Score and rank results
  - [x] Return `SearchResults` struct
- [x] Define `SearchResults` struct
  - [x] `results`: Vec<SearchResult>
  - [x] `total_count`: total matches (before limit)
  - [x] `query`: echo back query for reference
- [x] Define `SearchResult` struct
  - [x] `task_id`, `title`, `file_path`, `line`
  - [x] `score`: relevance score
  - [x] `snippet`: context snippet with highlighted terms
  - [x] `matched_fields`: which fields matched (title, body, etc.)
- [x] Implement query parsing
  - [x] Support quoted phrases: `"exact match"`
  - [x] Support field filters: `label:backend`, `path:core/`
  - [x] Support boolean operators: `foo AND bar`, `foo OR bar` (if FTS5)
  - [x] Fallback to simple tokenization if operators not supported
- [x] Add error handling
  - [x] Invalid query syntax
  - [x] Empty query (return all tasks?)
  - [x] Index not built (suggest `lash index`)

### Success Criteria

- Search API is easy to use from CLI command
- Supports common query patterns
- Fast: <200ms for typical queries
- Returns comprehensive result metadata

### Tests

- Unit: Test query parsing
- Unit: Test filter application
- Integration: Search fixture project with various queries
- Integration: Test pagination (offset + limit)
- Performance: Benchmark query time for different project sizes

---

## Task 4: Search Performance Optimization

**Priority:** MEDIUM
**Effort:** 1-2 days
**Depends on:** Task 3

### Description

Profile and optimize search performance for large projects.

### Subtasks

- [x] Add performance instrumentation
  - [x] Measure query execution time
  - [x] Measure scoring time
  - [x] Measure result formatting time (snippet generation)
- [x] Optimize bottlenecks
  - [-] Use prepared statements for FTS5 queries (deferred - not needed for current performance)
  - [-] Cache frequently used queries (deferred - not needed, already very fast)
  - [x] Optimize snippet extraction (pre-allocate capacity, avoid redundant allocations)
  - [-] Parallelize scoring (not beneficial for current result set sizes)
- [-] Tune FTS5 configuration (deferred - current config meets targets)
  - [-] Experiment with different tokenizers
  - [-] Adjust column weights
  - [-] Enable/disable stemming
- [-] Add result caching (optional - deferred, not needed given current performance)
  - [-] Cache recent queries in memory
  - [-] Invalidate on index changes
  - [-] Configurable cache size
- [x] Benchmark and document
  - [x] Small project (100 tasks): <50ms (achieved: ~0.5ms)
  - [x] Medium project (1000 tasks): <150ms (achieved: ~2.6ms)
  - [-] Large project (10000 tasks): <500ms (deferred - extrapolated performance well under target)

### Success Criteria

- Search meets performance targets
- Bottlenecks identified and resolved
- Caching improves repeat query performance

### Tests

- Benchmark: Generate projects of various sizes
- Benchmark: Measure query time for each size
- Benchmark: Test cache hit/miss performance

---

## Task 5: Search Filters Integration

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** Task 3

### Description

Integrate search with existing filter options (labels, status, path).

### Subtasks

- [x] Extend `SearchQuery` to accept filters
  - [x] `labels`: Vec<String>
  - [x] `status`: Option<TaskStatus>
  - [x] `path`: Option<PathBuf>
- [x] Implement filter application
  - [x] Combine FTS5 query with SQL WHERE clauses
  - [x] Filter results after FTS5 query if needed
  - [x] Maintain ranking order while filtering
- [x] Add filter query syntax (optional)
  - [x] `label:backend query text`
  - [x] `status:open query text`
  - [x] `path:core/ query text`
  - [x] Parse and apply filters from query string
- [x] Test filter combinations
  - [x] Search + label filter
  - [x] Search + status filter
  - [x] Search + multiple filters

### Success Criteria

- Filters work correctly with search
- Results match both query and filters
- Performance not significantly impacted by filters

### Tests

- Integration: Search with label filter
- Integration: Search with status filter
- Integration: Search with path filter
- Integration: Search with multiple filters

---

## Non-Goals (for v1)

- Advanced query syntax (complex boolean expressions)
- Faceted search (aggregations by field)
- Search suggestions / autocomplete
- Fuzzy typo correction (basic fuzzy matching is enough)
- Search history tracking

---

## Open Questions

- **FTS5 vs fuzzy matcher:** Which provides better UX? (Recommend FTS5 for simplicity)
- **Stemming:** Enable for better recall or disable for precise matching?
- **Snippet length:** How many characters of context? (Recommend 100-200 chars)
- **Highlighting format:** How to indicate matched terms in output? (Use ANSI colors for terminal, <mark> for JSON)

---

## References

- Design doc section 7.3.3 (`lash search` command)
- Design doc section 9.3 (Fuzzy search approaches)
- SQLite FTS5 docs: https://www.sqlite.org/fts5.html
- Fuzzy matching libraries:
  - fuzzy-matcher: https://docs.rs/fuzzy-matcher/
  - sublime_fuzzy: https://docs.rs/sublime_fuzzy/
