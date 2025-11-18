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

- [ ] Research FTS5 vs in-memory fuzzy matching
  - [ ] Benchmark FTS5 query performance
  - [ ] Evaluate fuzzy matching libraries (fuzzy-matcher, sublime_fuzzy)
  - [ ] Consider hybrid approach: FTS5 + in-memory ranking
- [ ] Define FTS5 virtual table (if using FTS5)
  - [ ] `search_index` table with columns:
    - [ ] `task_id` (FK to tasks.id)
    - [ ] `content` (combined searchable text)
    - [ ] `title` (task title, higher weight)
    - [ ] `body` (task body, lower weight)
    - [ ] `labels` (space-separated labels)
    - [ ] `file_path` (for filename matching)
  - [ ] Configure tokenizer (unicode61 or porter)
  - [ ] Set up column weights (title > labels > body)
- [ ] Implement search index population
  - [ ] During indexing, populate FTS5 table
  - [ ] Extract text from tasks: title + body + labels + path
  - [ ] Insert into search_index
- [ ] Add index maintenance
  - [ ] Update search index when tasks change
  - [ ] Delete from index when tasks deleted
  - [ ] Rebuild index command

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

- [ ] Define scoring algorithm
  - [ ] Base score from FTS5 ranking (bm25 or similar)
  - [ ] Boost for exact prefix matches
  - [ ] Boost for title matches vs body matches
  - [ ] Penalty for long documents (normalize)
- [ ] Implement `SearchScorer` struct
  - [ ] Take query and matched document
  - [ ] Compute relevance score (0.0 to 1.0)
  - [ ] Support multiple ranking strategies
- [ ] Add term highlighting
  - [ ] Identify matched terms in results
  - [ ] Store match positions for highlighting
  - [ ] Support context snippets (show surrounding text)
- [ ] Implement result ranking
  - [ ] Sort by score descending
  - [ ] Break ties by created date or path
  - [ ] Support pagination (offset + limit)
- [ ] Tune scoring weights
  - [ ] Experiment with different weight combinations
  - [ ] Test against representative queries
  - [ ] Document tuning rationale

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

- [ ] Define `SearchQuery` struct
  - [ ] `query`: search string
  - [ ] `scope`: optional path filter
  - [ ] `limit`: max results (default 20)
  - [ ] `offset`: pagination offset (default 0)
  - [ ] `filters`: label, status, etc.
- [ ] Implement `search()` function
  - [ ] Parse query string
  - [ ] Build FTS5 query (or run fuzzy matcher)
  - [ ] Apply scope and filters
  - [ ] Execute search
  - [ ] Score and rank results
  - [ ] Return `SearchResults` struct
- [ ] Define `SearchResults` struct
  - [ ] `results`: Vec<SearchResult>
  - [ ] `total_count`: total matches (before limit)
  - [ ] `query`: echo back query for reference
- [ ] Define `SearchResult` struct
  - [ ] `task_id`, `title`, `file_path`, `line`
  - [ ] `score`: relevance score
  - [ ] `snippet`: context snippet with highlighted terms
  - [ ] `matched_fields`: which fields matched (title, body, etc.)
- [ ] Implement query parsing
  - [ ] Support quoted phrases: `"exact match"`
  - [ ] Support field filters: `label:backend`, `path:core/`
  - [ ] Support boolean operators: `foo AND bar`, `foo OR bar` (if FTS5)
  - [ ] Fallback to simple tokenization if operators not supported
- [ ] Add error handling
  - [ ] Invalid query syntax
  - [ ] Empty query (return all tasks?)
  - [ ] Index not built (suggest `lash index`)

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

- [ ] Add performance instrumentation
  - [ ] Measure query execution time
  - [ ] Measure scoring time
  - [ ] Measure result formatting time
- [ ] Optimize bottlenecks
  - [ ] Use prepared statements for FTS5 queries
  - [ ] Cache frequently used queries (LRU cache)
  - [ ] Optimize snippet extraction (avoid full text fetch)
  - [ ] Parallelize scoring if beneficial
- [ ] Tune FTS5 configuration
  - [ ] Experiment with different tokenizers
  - [ ] Adjust column weights
  - [ ] Enable/disable stemming
- [ ] Add result caching (optional)
  - [ ] Cache recent queries in memory
  - [ ] Invalidate on index changes
  - [ ] Configurable cache size
- [ ] Benchmark and document
  - [ ] Small project (100 tasks): <50ms
  - [ ] Medium project (1000 tasks): <150ms
  - [ ] Large project (10000 tasks): <500ms

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

- [ ] Extend `SearchQuery` to accept filters
  - [ ] `labels`: Vec<String>
  - [ ] `status`: Option<TaskStatus>
  - [ ] `path`: Option<PathBuf>
- [ ] Implement filter application
  - [ ] Combine FTS5 query with SQL WHERE clauses
  - [ ] Filter results after FTS5 query if needed
  - [ ] Maintain ranking order while filtering
- [ ] Add filter query syntax (optional)
  - [ ] `label:backend query text`
  - [ ] `status:open query text`
  - [ ] `path:core/ query text`
  - [ ] Parse and apply filters from query string
- [ ] Test filter combinations
  - [ ] Search + label filter
  - [ ] Search + status filter
  - [ ] Search + multiple filters

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
