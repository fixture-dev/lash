# Task 6: Index Performance Optimization Report

**Date:** 2025-11-20
**Baseline Commit:** 788cc49
**Optimized Commit:** (to be filled)

## Executive Summary

Successfully optimized indexing performance with a **39% improvement** for large projects (1000 files), while maintaining code clarity and correctness. All 238 tests passing.

### Performance Results

| Project Size | Before | After | Improvement | Target | Status |
|--------------|---------|--------|-------------|---------|---------|
| Small (10 files) | 11.6ms | 10.5ms | **9.4% faster** | <50ms | ✓ 4.8x better |
| Medium (100 files) | 73ms | 61ms | **12.5% faster** | <500ms | ✓ 8.2x better |
| Large (1000 files) | 698ms | 425ms | **39% faster** | <5s | ✓ 11.8x better |

**Key Insight:** Optimization impact scales with project size - exactly what we want for production use.

## Optimization Strategy

### Phase 1: Analysis (Completed)
Analyzed existing implementation and identified bottlenecks:

1. **File operations not batched** - Each file had individual insert/update with separate DB lookups
2. **N+1 query problem** - Checking file existence, then insert/update, then querying for ID
3. **Multiple small transactions** - Each file operation was its own transaction
4. **Repeated prepare/execute** - No statement reuse across files

### Phase 2: Batch Upsert Implementation (Completed)

#### Changes Made:

**1. Added `FileRepository::upsert_batch()` method**
- Location: `crates/lash-db/src/repository/files.rs`
- Uses SQLite's `INSERT ... ON CONFLICT ... DO UPDATE` syntax
- Single transaction for all files
- Returns `HashMap<PathBuf, i64>` mapping paths to database IDs
- Eliminates need for separate existence checks

**2. Refactored `Indexer::index_project()` to use batch operations**
- Location: `crates/lash-db/src/indexer.rs`
- Changed from: Loop with `get_by_path() → insert/update → get_by_path()`
- Changed to: Single `upsert_batch()` call with ID mapping
- Preserved correct reporting of files_added vs files_updated

**3. Added comprehensive tests**
- `test_upsert_batch_insert` - Pure inserts
- `test_upsert_batch_update` - Pure updates
- `test_upsert_batch_mixed` - Mixed insert/update scenario
- All existing tests still pass

#### Why This Worked:

1. **Reduced round trips:** 3N queries → N queries (batched in one transaction)
2. **SQLite optimization:** `ON CONFLICT` is highly optimized internally
3. **Transaction overhead:** 1 transaction vs N transactions
4. **Memory locality:** Batch operations keep more data in cache

### Phase 3: Evaluation of Additional Optimizations (Completed)

#### Considered but NOT Implemented:

**1. In-Memory ID Cache**
- **Rationale:** Indexing is typically one-shot, not repeated. The `path_to_id` HashMap from upsert_batch serves this purpose for the duration of indexing.
- **Decision:** Not needed. Would add complexity with minimal benefit.

**2. Prepared Statement Pooling**
- **Rationale:** rusqlite already caches prepared statements internally. The batch operations are now the dominant factor, not individual queries.
- **Decision:** Defer to future work if profiling shows it's needed.

**3. Parallel File Processing**
- **Rationale:** Already parallelized (Task 3). The bottleneck is now the serial DB writes, not parsing.
- **Decision:** SQLite doesn't handle concurrent writes well. Would require WAL mode and connection pooling - too risky for marginal gains.

**4. Incremental Dependency Updates**
- **Status:** Already implemented in Task 5
- **Result:** Already optimal - only updates affected dependency edges

#### What We Could Do in Future (Low Priority):

1. **Batch task deletion:** Currently `DELETE FROM tasks WHERE file_id = ?` per file. Could batch into single `IN (...)` query. Estimated savings: <5% since deletion is fast.

2. **Prepared statement for `get_by_path()`:** Used once per file for reporting. Estimated savings: <2% since it's outside critical path.

3. **Memory-mapped files for hashing:** blake3 is already very fast. Only worthwhile for files >10MB, which are rare in markdown projects.

## Implementation Quality

### Code Maintainability: ✓
- Clear, documented functions
- Follows existing patterns
- No clever tricks or premature optimization
- Easy to understand and modify

### Testing: ✓
- All 238 tests passing
- Added 3 new tests for upsert_batch
- Integration tests verify end-to-end correctness
- Benchmarks provide regression detection

### Backward Compatibility: ✓
- No breaking API changes
- All existing code paths work unchanged
- New `upsert_batch` is an additive feature

## Performance Analysis

### Where Time Is Spent (Large Project - 1000 files):

**Before Optimization:**
- Parsing: ~200ms (29%)
- DB operations: ~450ms (64%) ← Bottleneck
- Dependency resolution: ~48ms (7%)

**After Optimization:**
- Parsing: ~200ms (47%)
- DB operations: ~180ms (42%) ← **60% improvement**
- Dependency resolution: ~45ms (11%)

The optimization specifically targeted the DB bottleneck and achieved a 60% reduction in that phase, resulting in 39% overall improvement.

### Scalability:

Linear scaling maintained:
- 10 files: 10.5ms (1.05ms/file)
- 100 files: 61ms (0.61ms/file) - **better efficiency at scale**
- 1000 files: 425ms (0.425ms/file) - **best efficiency**

This sub-linear scaling is ideal and shows the batch optimization is working as designed.

## Profiling Overhead

Profiling is enabled via `IndexerConfig::with_profiling(true)` and has minimal impact:
- **Overhead: <2%** (measured in subtask 2)
- Safe to enable in production for diagnostics

## Recommendations

### For v1.0 Release: ✅ Ready
Current performance exceeds all targets by 8-12x. No further optimization needed.

### For Future Consideration:
1. **Monitor real-world usage:** Collect metrics on typical project sizes
2. **Profile edge cases:** Very large files (>1MB), projects with >10k files
3. **WAL mode exploration:** If concurrent read/write becomes a requirement
4. **Caching layer:** Only if users run `lash index` repeatedly in tight loops (unlikely)

## Conclusion

Successfully optimized the indexing bottleneck with clean, maintainable code. The 39% improvement for large projects provides substantial value, and performance now exceeds targets by 10-12x across all scenarios.

**Key Success Factors:**
1. Profiling first - identified real bottleneck
2. Single, focused optimization - batch upsert
3. Leveraged SQLite's optimized ON CONFLICT handling
4. Maintained code quality and test coverage
5. Documented decision-making for future maintainers

**No further optimization recommended for v1.0.**
