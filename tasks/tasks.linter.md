# Linter Tasks

**Module:** Parsing & Validation
**Priority:** CRITICAL (syntax rules), HIGH (semantic rules)
**Estimated Duration:** 8-10 days
**Dependencies:** tasks.markdown-parser (all tasks)

## Overview

Implement the linter that validates Lash Markdown files for both syntax and semantic correctness. The linter enforces the strict, predictable format that makes Lash agent-friendly.

**Key Principle:** Linter provides rich, actionable diagnostics with auto-fix suggestions where possible.

**Design:** Rule-based architecture where each rule is independent and can be enabled/disabled.

## Tasks

### 1. Define Linter Rules Engine

- [x] **Create `LintRule` trait in `lash-core/src/linter/rule.rs`**
  - [x] Methods:
    - [x] `fn code(&self) -> &'static str` - Stable rule code (e.g., "E_DEPTH_EXCEEDED")
    - [x] `fn severity(&self) -> Severity` - Error, Warning, or Info
    - [x] `fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<Diagnostic>`
    - [x] `fn check_task(&self, task: &Task, ctx: &LintContext) -> Vec<Diagnostic>`
  - [x] Some rules apply to whole file, some to individual tasks
- [x] **Define `Diagnostic` struct** (extend from error-handling)
  - [x] Fields:
    - [x] `code: &'static str` - Rule code
    - [x] `severity: Severity`
    - [x] `message: String`
    - [x] `location: Location` - File, line, column, span
    - [x] `suggestion: Option<Fix>` - Auto-fix if available
  - [x] Methods:
    - [x] `to_human_string() -> String` - Colored, formatted for terminal (via Display trait)
    - [x] `to_json() -> String` - Machine-readable format
- [x] **Define `Fix` struct for auto-fixes**
  - [x] Fields:
    - [x] `description: String` - What the fix does
    - [x] `replacement: Replacement` - Text replacement or operation
  - [x] Support types:
    - [x] Text replacement (old → new)
    - [x] Insertion (at position)
    - [x] Deletion (range)
    - [x] Reformat (whole file)
- [x] **Create `Linter` struct**
  - [x] Fields:
    - [x] `rules: Vec<Arc<dyn LintRule>>` - Registered rules (using Arc for shared ownership)
    - [x] `config: LintConfig` - Enable/disable rules, severity overrides
  - [x] Methods:
    - [x] `new(config: LintConfig) -> Self`
    - [x] `register_rule(rule: Arc<dyn LintRule>)` - Add rule
    - [x] `lint_file(file: &TaskFile) -> Vec<Diagnostic>` - Run all rules
    - [x] `lint_project(files: &HashMap<PathBuf, TaskFile>) -> Vec<Diagnostic>` - Lint all files
- [x] **Create `LintContext` for rule execution**
  - [x] Provides shared data to rules:
    - [x] `config: &LashConfig` - Project config
    - [x] `file_path: PathBuf` - Current file
    - [x] `all_files: &HashMap<PathBuf, TaskFile>` - For cross-file validation
    - [x] Method: `is_annotation_allowed(&self, key: &str)` - Validates custom annotations
- [x] **Define `LintConfig` for linter configuration**
  - [x] Fields:
    - [x] `enabled_rules: HashSet<String>` - Which rules to run
    - [x] `disabled_rules: HashSet<String>` - Which to skip
    - [x] `severity_overrides: HashMap<String, Severity>` - Override default severity
    - [x] `auto_fix: bool` - Apply fixes automatically
  - [x] Serde support for loading from `.lash/config.toml` under `[linter]` section
- [x] **Create rule registry**
  - [x] Struct `RuleRegistry` with category-based organization
  - [x] Function `register_default_rules() -> RuleRegistry` - Returns registry (rules will be added in Task #2/#3)
  - [x] Organized by category: syntax, semantic, cross-file

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** tasks.markdown-parser#6
**Success Criteria:** Rule engine works; can run rules and collect diagnostics

---

### 2. Implement Syntax Rules

- [x] **Rule: Valid Checkbox Pattern**
  - [x] Code: `E_SYNTAX_CHECKBOX`
  - [x] Check: `- [X]` where X is ` `, `x`, `X`, `-`, or `!`
  - [x] Error on: `- []`, `- [ x]` (extra space), `- [v]`, etc.
  - [x] Suggestion: Show valid patterns
  - [x] Auto-fix: None (ambiguous intent)
  - [x] Note: Validation performed at parse time (see rule documentation)
- [x] **Rule: Consistent Indentation**
  - [x] Code: `E_SYNTAX_INDENT`
  - [x] Check: All checkbox lines use exactly 2 spaces per level
  - [x] Error on: Tabs, 4 spaces, 3 spaces, mixed
  - [x] Suggestion: "Use 2 spaces per indentation level"
  - [x] Auto-fix: Normalize to 2 spaces (calculate depth, reformat)
  - [x] Note: Validation performed at parse time (see rule documentation)
- [x] **Rule: Depth Limit**
  - [x] Code: `E_SYNTAX_DEPTH`
  - [x] Check: Task depth ≤ 2 (3 levels: 0, 1, 2)
  - [x] Error on: Depth 3+ (6+ spaces of indentation)
  - [x] Suggestion: "Split deep hierarchies into separate files"
  - [x] Auto-fix: None (requires restructuring)
- [x] **Rule: Valid Annotation Syntax**
  - [x] Code: `E_SYNTAX_ANNOTATION`
  - [x] Check: Lines starting with `@` match `@key: value` format
  - [x] Error on: `@key value` (no colon), `@ key: value` (space after @)
  - [x] Suggestion: Show correct format with example
  - [x] Auto-fix: Add missing colon if detectable
  - [x] Note: Validation performed at parse time (see rule documentation)
- [x] **Rule: Unknown Annotation Keys**
  - [x] Code: `E_SYNTAX_UNKNOWN_KEY`
  - [x] Check: `@key` is in built-in list OR custom_keys from config
  - [x] Built-in: id, labels, status, owner, created, estimate, depends-on, agent-note
  - [x] Error on: Unknown key not in either list
  - [x] Suggestion: "Add to .lash/config.toml [annotations.custom_keys] or fix typo"
  - [x] Auto-fix: None (ambiguous whether typo or intentional)
  - [x] Include fuzzy match suggestions for likely typos
- [x] **Rule: Header Structure**
  - [x] Code: `W_SYNTAX_HEADER`
  - [x] Check: File has H1 title and "## Tasks" section
  - [x] Warning on: Missing H1, missing Tasks section
  - [x] Suggestion: Add required sections
  - [x] Auto-fix: Insert template header structure
  - [x] Note: Only checks H1 title with parsed structure; "## Tasks" check requires raw content
- [x] **Rule: Annotation Ordering**
  - [x] Code: `I_SYNTAX_ORDER` (Info severity)
  - [x] Check: Annotations in alphabetical order (optional style)
  - [x] Info on: Out of order
  - [x] Suggestion: "Consider sorting for consistency"
  - [x] Auto-fix: Sort annotations alphabetically
  - [x] Note: Requires raw content access; deferred to formatter
- [x] **Write tests for each rule**
  - [x] Valid cases (should pass)
  - [x] Invalid cases (should fail)
  - [x] Edge cases
  - [x] Auto-fix application
  - [x] 30 tests total across all 7 rules (comprehensive coverage)

**Priority:** CRITICAL
**Estimate:** 2 days
**Dependencies:** Task #1
**Success Criteria:** All syntax rules implemented; catches formatting errors; provides fixes

---

### 3. Implement Semantic Rules

- [x] **Rule: ID Uniqueness Within File**
  - [x] Code: `E_SEM_DUPLICATE_ID`
  - [x] Check: No two tasks in same file have same ID
  - [x] Error on: Duplicate IDs
  - [x] Provide line numbers for all occurrences
  - [x] Suggestion: "Rename one of these tasks"
  - [x] Auto-fix: None (ambiguous which to keep)
- [x] **Rule: Parent-Child Status Consistency**
  - [x] Code: `W_SEM_STATUS_INCONSISTENT`
  - [x] Check: Parent cannot be Done if children are Open
  - [x] Warning on: Parent marked [x] but has [ ] children
  - [x] Suggestion: "Complete all children first, or waive them"
  - [x] Auto-fix: Option 1: Unmark parent; Option 2: Mark children
  - [x] Default auto-fix: Unmark parent (safer)
- [x] **Rule: Waived Children (auto-fix)**
  - [x] Code: `I_SEM_AUTO_WAIVE`
  - [x] Check: Parent is Waived
  - [x] Action: Auto-waive all children (per design decision)
  - [x] Info severity: "Auto-waiving children due to waived parent"
  - [x] Auto-fix: Set all descendant status to Waived
  - [x] Always apply in formatter
- [x] **Rule: Valid Label Format**
  - [x] Code: `E_SEM_INVALID_LABEL`
  - [x] Check: Labels match pattern: `[a-z0-9][a-z0-9-_]*`
  - [x] Error on: Uppercase, spaces, special chars, starts with number
  - [x] Suggestion: "Labels must be lowercase alphanumeric with - or _"
  - [x] Auto-fix: Normalize label (lowercase, replace spaces with -)
- [x] **Rule: Valid Date Format**
  - [x] Code: `E_SEM_INVALID_DATE`
  - [x] Check: `@created` matches YYYY-MM-DD
  - [x] Error on: Other formats, invalid dates (Feb 30, etc.)
  - [x] Suggestion: "Use YYYY-MM-DD format"
  - [x] Auto-fix: Attempt to parse and reformat if possible
- [x] **Rule: Valid Estimate Format**
  - [x] Code: `E_SEM_INVALID_ESTIMATE`
  - [x] Check: `@estimate` matches pattern: `\d+[hdwmy]`
  - [x] h=hours, d=days, w=weeks, m=months, y=years
  - [x] Error on: Invalid format
  - [x] Suggestion: "Use format like: 2h, 3d, 1w"
  - [x] Auto-fix: None (ambiguous conversion)
- [x] **Rule: Valid Owner Format**
  - [x] Code: `W_SEM_OWNER_FORMAT`
  - [x] Check: `@owner` is non-empty, reasonable length
  - [x] Warning on: Very long names (>100 chars)
  - [x] Suggestion: "Owner name seems unusually long"
  - [x] Auto-fix: Trim to reasonable length
- [x] **Rule: Empty Task Title**
  - [x] Code: `E_SEM_EMPTY_TITLE`
  - [x] Check: Task title is not empty or whitespace-only
  - [x] Error on: `- [ ]` with no text
  - [x] Suggestion: "Tasks must have a title"
  - [x] Auto-fix: None (needs content)
- [x] **Write tests for each rule**
  - [x] Valid cases
  - [x] Invalid cases (duplicates, inconsistencies, format errors)
  - [x] Auto-fix application
  - [x] Complex scenarios (parent-child chains)
  - [x] 5-10 tests per rule = 40-80 tests total (69 tests written)

**Priority:** HIGH
**Estimate:** 2 days
**Dependencies:** Task #1
**Success Criteria:** Semantic rules catch logical errors; status consistency enforced

---

### 4. Implement Cross-File Validation

- [x] **Rule: Dependency Reference Exists**
  - [x] Code: `E_LINK_NOT_FOUND`
  - [x] Check: `@depends-on` targets exist
  - [x] For file refs: Check file exists in project
  - [x] For task refs: Check file exists AND contains task ID
  - [x] Error on: Broken references
  - [x] Provide: Path to missing file/task
  - [x] Suggestion: Check path spelling, or create the file/task
  - [x] Auto-fix: None (can't auto-create tasks)
- [ ] **Rule: Circular Dependencies**
  - [ ] Code: `E_LINK_CYCLE`
  - [ ] Check: No cycles in dependency graph
  - [ ] Build graph from all `@depends-on` annotations
  - [ ] Run cycle detection (DFS with visited set)
  - [ ] Error on: Any cycle detected
  - [ ] Provide: Full cycle path (A → B → C → A)
  - [ ] Suggestion: "Remove one dependency to break cycle"
  - [ ] Auto-fix: None (ambiguous which edge to remove)
- [ ] **Rule: Root Index File References**
  - [ ] Code: `E_INDEX_FILE_MISSING`
  - [ ] Check: Files referenced in root index exist
  - [ ] Parse root index checkbox list
  - [ ] Verify each referenced file path exists
  - [ ] Error on: Missing files
  - [ ] Suggestion: "Create file or remove from index"
  - [ ] Auto-fix: Option to remove from index
- [ ] **Rule: Orphaned Files**
  - [ ] Code: `W_INDEX_ORPHAN`
  - [ ] Check: All .md files in project are in root index
  - [ ] Warning on: Files not referenced in index
  - [ ] Suggestion: "Add to lash.index.md or move to archive"
  - [ ] Auto-fix: Add to index under appropriate section
- [ ] **Rule: Valid Dependency Path Resolution**
  - [ ] Code: `E_LINK_INVALID_PATH`
  - [ ] Check: Relative paths resolve correctly
  - [ ] `../core/api.md` from `tasks/ui/login.md` resolves
  - [ ] Error on: Paths escaping project root
  - [ ] Error on: Malformed paths (double //, etc.)
  - [ ] Suggestion: Fix path syntax
  - [ ] Auto-fix: Normalize path separators
- [ ] **Implement cross-file context**
  - [ ] `LintContext` includes all parsed files
  - [ ] Build dependency graph across project
  - [ ] Cache file lookups for performance
  - [ ] Support incremental linting (only check changed files' deps)
- [ ] **Write tests**
  - [ ] Valid cross-file dependencies
  - [ ] Broken file references
  - [ ] Broken task references
  - [ ] Circular dependencies (various patterns)
  - [ ] Index validation
  - [ ] Orphaned files detection
  - [ ] 25+ tests for cross-file scenarios

**Priority:** MEDIUM (can lint single files without this)
**Estimate:** 2 days
**Dependencies:** tasks.dependency-resolution#1 (graph building)
**Success Criteria:** Detects broken references and cycles; validates index

---

### 5. Implement Auto-Formatter

- [ ] **Create `Formatter` struct in `lash-core/src/formatter/`**
  - [ ] Fields:
    - [ ] `config: LashConfig`
    - [ ] `format_options: FormatOptions`
  - [ ] Methods:
    - [ ] `format_file(&self, file: &TaskFile) -> String` - Format to string
    - [ ] `format_file_in_place(&self, path: &Path) -> Result<()>` - Write back
- [ ] **Define `FormatOptions`**
  - [ ] `indent_spaces: u8` - Default 2
  - [ ] `sort_annotations: bool` - Default true
  - [ ] `normalize_whitespace: bool` - Default true
  - [ ] `apply_auto_fixes: bool` - Default true (waiving, status consistency)
  - [ ] `preserve_blank_lines: bool` - How many to keep (max 2)
- [ ] **Normalize indentation**
  - [ ] Convert all task indentation to exactly 2 spaces per level
  - [ ] Remove tabs, convert to spaces
  - [ ] Maintain correct depth hierarchy
  - [ ] Preserve non-task content indentation (code blocks, etc.)
- [ ] **Sort annotations alphabetically**
  - [ ] Within header block, sort `@key:` lines by key name
  - [ ] Keep `@id` first always (special case)
  - [ ] Preserve annotation comments if any
  - [ ] Maintain multiline value formatting
- [ ] **Normalize whitespace**
  - [ ] Trim trailing whitespace from all lines
  - [ ] Ensure single blank line between sections
  - [ ] Collapse multiple blank lines to max 2
  - [ ] Ensure file ends with single newline
  - [ ] Trim leading/trailing whitespace from annotation values
- [ ] **Apply auto-fixes**
  - [ ] Run all linter rules with auto-fix enabled
  - [ ] Collect fixes from diagnostics
  - [ ] Apply in order (be careful with overlapping fixes)
  - [ ] Re-parse to verify output is valid
  - [ ] Error if formatting breaks parsing
- [ ] **Preserve non-task content**
  - [ ] Keep overview text unchanged
  - [ ] Keep references section unchanged
  - [ ] Preserve markdown formatting (bold, italic, links)
  - [ ] Preserve code blocks verbatim
  - [ ] Only format task structure and annotations
- [ ] **Implement dry-run mode**
  - [ ] `--dry-run` flag shows what would change
  - [ ] Output diff-style (- old, + new)
  - [ ] Don't modify files
  - [ ] Exit 0 if no changes, 1 if would change
- [ ] **Ensure round-trip safety**
  - [ ] parse → format → parse should be idempotent
  - [ ] Content semantics preserved
  - [ ] No data loss
  - [ ] Add round-trip tests
- [ ] **Write comprehensive tests**
  - [ ] Format messy file (bad indent, whitespace)
  - [ ] Sort annotations
  - [ ] Apply auto-fixes (waiving, etc.)
  - [ ] Round-trip tests (format is idempotent)
  - [ ] Preserve non-task content
  - [ ] Dry-run mode
  - [ ] Files with various issues
  - [ ] 30+ tests

**Priority:** MEDIUM
**Estimate:** 2 days
**Dependencies:** Tasks #2, #3 (need rules to auto-fix)
**Success Criteria:** Can format files without data loss; idempotent formatting

---

### 6. Implement CLI Integration

- [ ] **Implement `lash lint` command in `lash-cli`**
  - [ ] Command structure:
    - [ ] `lash lint [PATH...]` - Lint specific files or directories
    - [ ] `lash lint` (no args) - Lint entire project from root
  - [ ] Options:
    - [ ] `--json` - Output JSON diagnostics
    - [ ] `--fix` - Apply auto-fixes
    - [ ] `--rule <code>` - Run only specific rule(s)
    - [ ] `--severity <level>` - Only show errors of this severity or higher
    - [ ] `--no-color` - Disable colored output
  - [ ] Exit codes:
    - [ ] 0 - No errors
    - [ ] 1 - General error (file not found, etc.)
    - [ ] 2 - Lint errors found
- [ ] **Implement `lash format` command**
  - [ ] Command structure:
    - [ ] `lash format [PATH...]` - Format specific files
    - [ ] `lash format` (no args) - Format entire project
  - [ ] Options:
    - [ ] `--check` - Check formatting without modifying (dry-run)
    - [ ] `--diff` - Show diff of changes
    - [ ] `--no-fix` - Only normalize formatting, don't apply lint fixes
  - [ ] Exit codes:
    - [ ] 0 - All files properly formatted (or successfully formatted)
    - [ ] 1 - General error
    - [ ] 2 - Files need formatting (with --check)
- [ ] **Implement progress reporting**
  - [ ] Show progress bar for multi-file operations
  - [ ] "Linting file.md..." spinner
  - [ ] Summary: "Checked 42 files, found 7 errors"
  - [ ] Use `indicatif` for progress bars
- [ ] **Format diagnostic output (human-readable)**
  - [ ] Format: `path/to/file.md:line:col: error[CODE]: message`
  - [ ] Example: `tasks/api.md:42:3: error[E_SYNTAX_DEPTH]: Task depth exceeds maximum (3 > 2)`
  - [ ] Color code by severity:
    - [ ] Red for errors
    - [ ] Yellow for warnings
    - [ ] Blue for info
  - [ ] Show code snippet with error location marked
  - [ ] Show suggestion/fix if available
- [ ] **Format diagnostic output (JSON)**
  - [ ] Schema:
    ```json
    {
      "diagnostics": [
        {
          "code": "E_SYNTAX_DEPTH",
          "severity": "error",
          "message": "Task depth exceeds maximum",
          "location": {
            "file": "tasks/api.md",
            "line": 42,
            "column": 3,
            "span": { "start": 120, "end": 145 }
          },
          "suggestion": "Split deep hierarchies into separate files",
          "fix": null
        }
      ],
      "summary": {
        "files_checked": 42,
        "errors": 7,
        "warnings": 3,
        "info": 1
      }
    }
    ```
  - [ ] Stable field names for machine parsing
  - [ ] Include all diagnostics, not just first few
- [ ] **Implement file discovery**
  - [ ] If path is directory, find all `.md` files recursively
  - [ ] Respect `.gitignore` patterns
  - [ ] Respect `.lashignore` if present
  - [ ] Sort files for deterministic output
- [ ] **Add `--fix` mode implementation**
  - [ ] Run linter to collect diagnostics
  - [ ] Filter for diagnostics with auto-fix available
  - [ ] Apply fixes to files
  - [ ] Re-lint to verify fixes worked
  - [ ] Report what was fixed
  - [ ] Warn if any fixes failed to apply
- [ ] **Write integration tests**
  - [ ] Run lint on valid file (exit 0, no output)
  - [ ] Run lint on invalid file (exit 2, show errors)
  - [ ] Run lint with --json (verify JSON schema)
  - [ ] Run lint with --fix (verify file changed)
  - [ ] Run format with --check (exit 2 if needs formatting)
  - [ ] Run format without --check (modify file)
  - [ ] Run on directory (multiple files)
  - [ ] Run with various option combinations
  - [ ] 20+ integration tests

**Priority:** HIGH
**Estimate:** 1 day
**Dependencies:** Tasks #2, #3, #5
**Success Criteria:** Commands work correctly; output is clear; exit codes correct

---

## Summary

### Total Estimate
**8-10 days** total for linter implementation

### Completion Criteria
- [ ] All tasks above completed
- [ ] 20+ linting rules implemented (syntax + semantic + cross-file)
- [ ] Auto-formatter works without data loss
- [ ] CLI commands functional with good UX
- [ ] 100+ unit tests for rules
- [ ] Integration tests for CLI
- [ ] JSON output for machine parsing

### Linting Rules Summary

**Syntax Rules (7):**
1. Valid checkbox pattern
2. Consistent indentation (2 spaces)
3. Depth limit enforcement (max depth 2)
4. Valid annotation syntax
5. Unknown annotation keys (strict with config)
6. Header structure
7. Annotation ordering (optional style)

**Semantic Rules (8):**
1. ID uniqueness within file
2. Parent-child status consistency
3. Auto-waive children (formatter rule)
4. Valid label format
5. Valid date format
6. Valid estimate format
7. Valid owner format
8. Non-empty task titles

**Cross-File Rules (5):**
1. Dependency references exist
2. No circular dependencies
3. Root index file references valid
4. Orphaned files detection
5. Valid path resolution

**Total: 20 rules** covering all aspects of file validity

### Architecture

```
LintRule (trait)
    ↓
Concrete Rules (20+)
    ↓
Linter (orchestrates rules)
    ↓
Diagnostic (reports issues)
    ↓
Fix (auto-fix suggestions)
    ↓
Formatter (applies fixes)
```

### Next Steps

After completing linter, proceed to:
1. **tasks.sqlite-schema.md** - Store validated data in database
2. Use linter in `lash index` to validate before indexing
3. Use formatter in pre-commit hooks
