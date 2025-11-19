# Parser Architecture

This document describes the architecture of the Lash Markdown parser.

## Overview

The Lash parser transforms Markdown task files into structured `TaskFile` objects. It's designed to be:

- **Fast**: Streaming event-based parsing targeting <100ms for typical files
- **Robust**: Continues after errors, collecting all diagnostics
- **Helpful**: Provides actionable error messages with precise location info

## Design Philosophy

### Streaming Over Tree-Based

We use `pulldown-cmark`'s streaming event-based API rather than constructing a full AST. This provides:

- **Lower memory usage**: No full tree in memory, only current parse state
- **Faster parsing**: Single-pass processing, no intermediate tree construction
- **Better error recovery**: Can skip malformed sections without losing entire structure

### Error Collection

The parser uses a "collect all errors" strategy:

- Continues parsing after encountering errors
- Returns all diagnostics at once
- Provides better UX than "stop at first error"
- Each error includes location, snippet, and fix suggestion

### Simplicity First

Code is optimized for clarity and maintainability:

- Simple, obvious algorithms over clever optimizations
- Clear separation of concerns across modules
- Extensive inline documentation
- Comprehensive test coverage

## Parsing Pipeline

The parser operates in four distinct phases:

```
┌─────────────────────────────────────────────────────────────┐
│                     Raw Markdown File                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Phase 1: Event Stream Processing (events.rs)               │
│                                                             │
│ • Convert pulldown-cmark events to semantic events         │
│ • Track line numbers and document structure                │
│ • Identify section boundaries (Header, Tasks, References)  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Phase 2: Line Parsing (checkbox.rs, annotations.rs)        │
│                                                             │
│ • Parse checkbox lines: "- [x] Task title #label"          │
│ • Extract: indentation, status, title, inline labels       │
│ • Parse annotations: "@id: task-123"                       │
│ • Extract: key-value pairs, handle multiline values        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Phase 3: Tree Building (builder.rs)                        │
│                                                             │
│ • Build parent-child relationships from indentation        │
│ • Validate depth limits and indentation consistency        │
│ • Generate synthetic IDs for tasks without @id             │
│ • Validate ID uniqueness within file                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Phase 4: File Construction (mod.rs)                        │
│                                                             │
│ • Combine header metadata + task tree + references         │
│ • Compute content hash (BLAKE3)                            │
│ • Return TaskFile or aggregated diagnostics                │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      TaskFile Object                        │
└─────────────────────────────────────────────────────────────┘
```

## Module Breakdown

### `mod.rs` - Main Parser Module

**Responsibility**: Orchestrate the parsing pipeline, manage parse context

**Key Types**:
- `ParseContext`: Tracks current parse state (file, line, section, errors, config)
- `ParseResult<T>`: Result type that can contain multiple diagnostics
- `ParsedFile`: Intermediate representation before final `TaskFile`
- `ParsedHeader`: Parsed header metadata and annotations
- `Section`: Enum for file sections (Header, Tasks, References, Other)

**Key Functions**:
- `parse_file(path, config) -> Result<TaskFile>`: Main entry point
- `parse_file_from_string(content, config) -> ParseResult<TaskFile>`: For testing

**Implementation Status**: Structure defined, main parsing logic in Task #6

### `events.rs` - Event Stream Processing

**Responsibility**: Convert pulldown-cmark events to parse actions

**Key Types**:
- `EventProcessor`: Stateful processor for Markdown event stream

**Key Functions**:
- `new(content)`: Create processor from Markdown content
- `next_event()`: Get next event, updating internal state
- State tracking: `current_line()`, `in_list()`, `list_depth()`, `heading_level()`

**Implementation Status**: Structure defined, will be used in Task #4 & #6

### `checkbox.rs` - Checkbox Line Parser

**Responsibility**: Parse individual checkbox task lines

**Key Types**:
- `CheckboxLine`: Intermediate representation of a parsed checkbox
  - `indent`: Number of leading spaces
  - `depth`: Computed nesting level (indent / 2)
  - `status`: TaskStatus (Open, Done, Waived, Blocked)
  - `title`: Task title text
  - `labels`: Inline labels parsed from title
  - `line_num`, `column`: Source location

**Key Functions**:
- `CheckboxLine::parse(line, line_num) -> Option<CheckboxLine>`: Parse a line
- `parse_inline_labels(title) -> Vec<Label>`: Extract #tags from title

**Validation**:
- Indentation must be multiple of 2 spaces
- Status must be one of: ` `, `x`, `X`, `-`, `!`
- Title cannot be empty

**Implementation Status**: Structure defined, parsing logic in Task #2

### `annotations.rs` - Annotation Parser

**Responsibility**: Parse metadata annotations

**Key Types**:
- `AnnotationBlock`: Collection of parsed annotations
  - Supports single-value annotations (@id, @owner)
  - Supports multi-value annotations (@depends-on, @labels)

**Key Functions**:
- `parse_annotation(line) -> Option<(key, value)>`: Parse `@key: value`
- `parse_inline_annotations(text) -> Option<AnnotationBlock>`: Parse `[@k: v, @k2: v2]`

**Known Annotations**:
- `@id`: Unique identifier
- `@labels`: Comma-separated labels
- `@status`: File/task status
- `@owner`: Assigned owner
- `@created`: Creation date (YYYY-MM-DD)
- `@estimate`: Time estimate
- `@depends-on`: Dependency reference (can appear multiple times)
- `@agent-note`: Note for LLM agents

**Implementation Status**: Structure defined, parsing logic in Task #3

### `builder.rs` - Task Tree Builder

**Responsibility**: Build hierarchical task tree from flat checkbox lines

**Key Types**:
- `TaskTreeBuilder`: Stateful builder for constructing task trees
  - Maintains parent stack for each depth level
  - Tracks used IDs for duplicate detection
  - Validates depth limits and indentation

**Key Algorithm** (Stack-based tree construction):
```
1. Initialize: parent_stack = [None]  (root level)
2. For each CheckboxLine:
   a. Validate: depth <= max_depth
   b. Validate: depth <= previous_depth + 1 (no skipping)
   c. Pop stack to current depth
   d. Parent = top of stack
   e. Create Task with parent reference
   f. Push Task onto stack
3. Build final TaskTree from flat task list
```

**Key Functions**:
- `new(max_depth) -> TaskTreeBuilder`: Create builder
- `add_line(checkbox) -> Result<()>`: Add a checkbox line
- `build() -> TaskTree`: Construct final tree
- `generate_synthetic_id(title, index) -> String`: Generate ID from title

**Validation**:
- Depth limit enforcement
- No skipped indentation levels
- ID uniqueness within file
- Valid parent-child relationships

**Implementation Status**: Structure defined, building logic in Task #5

## Error Handling Strategy

### Error Categories

All parse errors use the `LashError` type from `lash-types` with stable error codes:

**Parse Errors** (`E_PARSE_*`):
- `E_PARSE_INVALID_CHECKBOX`: Invalid checkbox syntax
- `E_PARSE_INVALID_ANNOTATION`: Malformed annotation
- `E_PARSE_INVALID_HEADER`: Invalid header format
- `E_PARSE_UNEXPECTED_DEPTH`: Unexpected indentation
- `E_PARSE_INVALID_DATE`: Invalid date format

**Lint Errors** (`E_LINT_*`):
- `E_LINT_DUPLICATE_ID`: Duplicate task ID
- `E_LINT_UNKNOWN_ANNOTATION`: Unknown annotation key
- `E_LINT_DEPTH_EXCEEDED`: Depth limit exceeded
- `E_LINT_BAD_INDENTATION`: Incorrect indentation
- `E_LINT_INVALID_LABEL`: Invalid label format

### Error Context

Every error includes:
- **Location**: File path, line number, column number
- **Message**: Clear description of the problem
- **Snippet**: Code snippet showing the error (when applicable)
- **Help**: Actionable suggestion for fixing the error
- **Code**: Stable error code for tooling integration

### Continue-on-Error

The parser continues after errors when possible:
- Collects all errors in `ParseContext.diagnostics`
- Returns complete error list at end
- Provides better UX than stopping at first error
- Enables "fix all" workflows in linters

## Performance Considerations

### Target Metrics

| File Size | Task Count | Target Time | Notes                    |
|-----------|------------|-------------|--------------------------|
| Small     | 10 tasks   | <100ms      | Pre-commit hook target   |
| Medium    | 100 tasks  | <500ms      | Reasonable for CLI use   |
| Large     | 1000 tasks | <5s         | Batch processing         |

### Optimization Strategies

1. **Streaming parsing**: No full AST construction
2. **Single-pass processing**: Where possible, process in one pass
3. **Minimal allocations**: Reuse strings, arena allocation for trees
4. **Early exit**: Stop on first error in strict mode (future)
5. **Lazy hash computation**: Only compute hash if needed

### Measurement

Use `criterion` benchmarks to track performance:
- Benchmark parsing at different file sizes
- Track regression across commits
- Profile hot paths if needed

## Testing Strategy

### Test Coverage Goals

- **100%** coverage of error paths (every error variant tested)
- **80%+** overall code coverage
- Edge cases for all parsing rules
- Round-trip tests (parse → format → parse)

### Test Layers

**Unit Tests** (inline in each module):
- Test individual parsing functions
- Test validation logic
- Test error cases
- Test edge cases

**Integration Tests** (mod.rs tests):
- Test complete parsing pipeline
- Test error collection
- Test valid and invalid files

**Fixture Tests** (future, in tests/ directory):
- Parse all files in `tests/fixtures/valid/`
- Parse all files in `tests/fixtures/invalid/` (expect errors)
- Verify error codes and messages

### Test Examples

See inline `#[cfg(test)]` modules in each file for examples.

## Extension Points

### Custom Annotations

The parser supports custom annotations via config:

```toml
# .lash/config.toml
[parser]
custom_annotations = ["priority", "epic", "sprint"]
```

Custom annotations are validated against this list if configured.

### Future Extensions

Potential areas for future enhancement:

1. **Incremental parsing**: Only re-parse changed sections
2. **Parallel parsing**: Parse multiple files concurrently
3. **Syntax recovery**: Better error recovery for malformed input
4. **AST caching**: Cache parsed AST for frequent re-parsing
5. **Fuzzy parsing**: Tolerate minor syntax variations

## Dependencies

- **pulldown-cmark 0.12**: Markdown parsing
- **lash-types**: Shared types, errors, configuration
- **regex**: Pattern matching (for labels, annotations)

## Next Steps

After completing Task #1 (this architecture), implement:

1. **Task #2**: Checkbox line parser
2. **Task #3**: Annotation parser
3. **Task #4**: Header block parser
4. **Task #5**: Task tree builder
5. **Task #6**: Full file parser integration

Each task builds on the previous, following the pipeline architecture described above.
