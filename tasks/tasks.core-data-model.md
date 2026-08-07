# Core Data Model Tasks

**Module:** Foundation & Infrastructure
**Priority:** CRITICAL
**Estimated Duration:** 5-7 days
**Dependencies:** tasks.project-setup#4 (error taxonomy)

## Overview

Define the core data structures that represent tasks, files, dependencies, and labels. These types are the foundation of the entire system and must be carefully designed.

**Key Design Principle:** Use arena allocation during parsing, convert to flat indexed representation for database storage.

## Tasks

### 1. Define Task Status Enum

- [x] **Create `TaskStatus` enum in `lash-types/src/status.rs`**
  - [x] Variants:
    - [x] `Open` - Task not yet started
    - [x] `Done` - Task completed
    - [x] `Waived` - Task marked as not applicable
    - [x] `Blocked` - Task cannot proceed (optional extension)
  - [x] Derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
- [x] **Implement Display trait**
  - [x] Format as human-readable: "open", "done", "waived", "blocked"
  - [x] Use lowercase for consistency
- [x] **Implement FromStr trait**
  - [x] Parse from strings: "open", "done", "waived", "blocked"
  - [x] Case-insensitive parsing
  - [x] Return error for unknown status strings
- [x] **Add checkbox character mapping**
  - [x] Create `to_checkbox_char(&self) -> char` method
    - [x] `Open` -> `' '` (space)
    - [x] `Done` -> `'x'`
    - [x] `Waived` -> `'-'`
    - [x] `Blocked` -> `'!'`
  - [x] Create `from_checkbox_char(c: char) -> Result<TaskStatus>` method
  - [x] Handle both uppercase and lowercase 'X'
- [x] **Add validation rules**
  - [x] Create `is_complete(&self) -> bool` method (Done or Waived)
  - [x] Create `requires_dependencies(&self) -> bool` method
  - [x] Document semantics in doc comments
- [x] **Write tests**
  - [x] Test round-trip: status -> char -> status
  - [x] Test string parsing (all variants, case-insensitive)
  - [x] Test display formatting
  - [x] Test error cases

**Priority:** CRITICAL
**Estimate:** 0.5 day
**Dependencies:** tasks.project-setup#4
**Success Criteria:** Can convert between status and checkbox chars; all tests pass

---

### 2. Implement Task Model

- [x] **Define `Task` struct in `lash-types/src/task.rs`**
  - [x] Fields:
    - [x] `id: String` - Unique ID within file (synthesized if not provided)
    - [x] `title: String` - Task title/description
    - [x] `status: TaskStatus` - Current status
    - [x] `depth: u8` - Nesting level (0 = top-level)
    - [x] `parent_id: Option<String>` - Parent task ID (if nested)
    - [x] `order_index: usize` - Position among siblings
    - [x] `metadata: TaskMetadata` - Optional annotations
    - [x] `body: Option<String>` - Extended description (optional)
  - [x] Derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] **Define `TaskMetadata` struct**
  - [x] Fields:
    - [x] `labels: Vec<String>` - Inline and explicit labels
    - [x] `owner: Option<String>` - Assignee
    - [x] `estimate: Option<String>` - Time estimate (e.g., "2h", "3d")
    - [x] `depends_on: Vec<DependencyRef>` - Explicit dependencies
    - [x] `agent_note: Option<String>` - Note for AI agents
    - [x] `custom: HashMap<String, String>` - Extensibility
  - [x] Derive `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
- [x] **Implement `Task` builder pattern**
  - [x] Create `TaskBuilder` struct
  - [x] Methods: `new(title)`, `status()`, `depth()`, `parent()`, `label()`, `build()`
  - [x] Validate depth limit in `build()`
  - [x] Generate ID if not provided: `task-{index}`
  - [x] Return errors for invalid configurations
- [x] **Add validation methods**
  - [x] `validate(&self, max_depth: u8) -> Result<()>`
    - [x] Check depth <= max_depth
    - [x] Check title not empty
    - [x] Check ID valid (alphanumeric, dash, underscore, colon)
    - [x] Check order_index >= 0
  - [x] `is_complete(&self) -> bool` - delegates to status
  - [x] `is_blocked(&self, deps: &[Dependency]) -> bool` - checks dependencies
- [x] **Implement hierarchical methods**
  - [x] `is_child_of(&self, parent_id: &str) -> bool`
  - [x] `depth_from_parent(&self) -> u8`
- [x] **Create `TaskTree` struct**
  - [x] Represent hierarchical task structure
  - [x] Use `Vec<Task>` internally (flat, indexed)
  - [x] Maintain parent-child mapping via IDs
  - [x] Methods:
    - [x] `add_task(&mut self, task: Task) -> Result<()>`
    - [x] `get_task(&self, id: &str) -> Option<&Task>`
    - [x] `get_children(&self, parent_id: &str) -> Vec<&Task>`
    - [x] `get_descendants(&self, id: &str) -> Vec<&Task>` (recursive)
    - [x] `validate(&self, max_depth: u8) -> Result<()>`
- [x] **Write tests**
  - [x] Test task creation via builder
  - [x] Test validation (depth limits, empty titles)
  - [x] Test parent-child relationships
  - [x] Test TaskTree operations (add, get, children)
  - [x] Test edge cases (cycles, missing parents)

**Priority:** CRITICAL
**Estimate:** 2 days
**Dependencies:** Task #1
**Success Criteria:** Can build task trees programmatically; validation works; all tests pass

---

### 3. Implement File Metadata Model

- [x] **Define `TaskFile` struct in `lash-types/src/file.rs`**
  - [x] Fields:
    - [x] `path: PathBuf` - Relative path from project root
    - [x] `title: String` - H1 title from file
    - [x] `id: String` - File identifier (from `@id` or derived from path)
    - [x] `metadata: FileMetadata` - File-level annotations
    - [x] `tasks: TaskTree` - Hierarchical task structure
    - [x] `hash: String` - Content hash (blake3)
    - [x] `mtime: SystemTime` - Last modified time
  - [x] Derive `Debug`, `Clone`
- [x] **Define `FileMetadata` struct**
  - [x] Fields:
    - [x] `labels: Vec<String>` - File-level labels
    - [x] `status: Option<String>` - Overall status
    - [x] `owner: Option<String>` - File owner
    - [x] `created: Option<String>` - Creation date (YYYY-MM-DD)
    - [x] `depends_on: Vec<DependencyRef>` - File-level dependencies
    - [x] `custom: HashMap<String, String>` - Other annotations
  - [x] Derive `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
- [x] **Implement content hashing**
  - [x] Add `blake3` dependency to `lash-types`
  - [x] Create `compute_hash(content: &str) -> String` function
  - [x] Hash the full file content
  - [x] Return hex-encoded hash string
  - [x] Create `hash_matches(&self, content: &str) -> bool` method
- [x] **Implement file-level validation**
  - [x] Create `validate(&self, config: &LashConfig) -> Result<()>` method
  - [x] Check file ID is unique (within project context, enforced elsewhere)
  - [x] Validate all tasks in tree
  - [x] Check for task ID uniqueness within file
  - [x] Validate dependency references (syntax only, not resolution)
- [x] **Implement file status computation**
  - [x] Create `compute_status(&self) -> FileStatus` method
    - [x] `Complete` if all top-level tasks complete
    - [x] `InProgress` if any top-level task in progress
    - [x] `Blocked` if any top-level task blocked
    - [x] `Empty` if no tasks
  - [x] Define `FileStatus` enum (Complete, InProgress, Blocked, Empty)
- [x] **Add ID synthesis**
  - [x] Create `synthesize_file_id(path: &Path) -> String` function
  - [x] Use dot-delimited path: `core.api.auth` from `core/api/auth.md`
  - [x] Strip `.md` extension
  - [x] Convert `/` to `.`
  - [x] Normalize to lowercase
- [x] **Write tests**
  - [x] Test hash computation and comparison
  - [x] Test ID synthesis from paths
  - [x] Test status computation
  - [x] Test validation (unique IDs, etc.)
  - [x] Test round-trip serialization

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Can represent complete task file in memory; hash/status computation works

---

### 4. Define Dependency Types

- [x] **Create `DependencyKind` enum in `lash-types/src/dependency.rs`**
  - [ ] Variants:
    - [ ] `Hierarchy` - Parent depends on child (implicit from nesting)
    - [ ] `ExplicitId` - `@depends-on: file-id#task-id`
    - [ ] `ExplicitPath` - `@depends-on: path/to/file.md`
    - [ ] `Directory` - File depends on subdirectory completion
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
- [x] **Define `DependencyRef` struct**
  - [ ] Fields:
    - [ ] `target: String` - Raw reference string
    - [ ] `kind: DependencyKind` - Parsed kind
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`
- [x] **Define `Dependency` struct (resolved)**
  - [ ] Fields:
    - [ ] `from_task_id: String` - Source task full ID (file-id#task-id)
    - [ ] `to_task_id: String` - Target task full ID
    - [ ] `kind: DependencyKind` - Dependency type
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
- [x] **Implement parsing for `@depends-on` references**
  - [ ] Create `parse_dependency_ref(s: &str) -> Result<DependencyRef>` function
  - [ ] Detect format:
    - [ ] File path: ends with `.md`
    - [ ] Full ID: contains `#` (split on first `#`)
    - [ ] Relative path: `../file.md` or `./file.md`
  - [ ] Store raw string and parsed kind
  - [ ] Validate syntax (not resolution)
- [x] **Create full task ID helpers**
  - [ ] Create `make_full_id(file_id: &str, task_id: &str) -> String`
  - [ ] Format: `{file_id}#{task_id}`
  - [ ] Create `parse_full_id(full_id: &str) -> Result<(String, String)>`
  - [ ] Split on first `#`
  - [ ] Return error if no `#` found
- [x] **Add validation**
  - [ ] Validate dependency reference syntax
  - [ ] Check for empty strings
  - [ ] Check for valid characters in IDs
  - [ ] Validate path format
- [x] **Add display formatting**
  - [ ] Implement `Display` for `DependencyRef`
  - [ ] Format based on kind:
    - [ ] `ExplicitId`: `file-id#task-id`
    - [ ] `ExplicitPath`: `path/to/file.md`
    - [ ] `Hierarchy`: `(implicit)`
    - [ ] `Directory`: `dir:path/to/dir/`
- [x] **Write tests**
  - [ ] Test parsing various reference formats
  - [ ] Test full ID creation and parsing
  - [ ] Test error cases (malformed references)
  - [ ] Test round-trip: ref -> parse -> display

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Can represent all dependency types; parsing works correctly

---

### 5. Implement Label Model

- [x] **Define `Label` struct in `lash-types/src/label.rs`**
  - [ ] Fields:
    - [ ] `name: String` - Normalized label name
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`
- [x] **Implement label parsing**
  - [ ] Create `parse_inline_labels(text: &str) -> Vec<Label>` function
    - [ ] Find `#word` patterns
    - [ ] Extract word after `#`
    - [ ] Normalize each label
    - [ ] Deduplicate
  - [ ] Create `parse_annotation_labels(text: &str) -> Vec<Label>` function
    - [ ] Parse `@labels: a, b, c` format
    - [ ] Split on commas
    - [ ] Trim whitespace
    - [ ] Normalize each label
    - [ ] Deduplicate
- [x] **Implement label normalization**
  - [ ] Create `normalize(s: &str) -> String` function
  - [ ] Convert to lowercase
  - [ ] Trim whitespace
  - [ ] Replace spaces with hyphens
  - [ ] Keep alphanumeric, hyphen, underscore only
  - [ ] Strip leading/trailing hyphens
- [x] **Implement validation**
  - [ ] Create `is_valid_label(s: &str) -> bool` function
  - [ ] Check length (1-50 characters)
  - [ ] Check allowed characters (alphanumeric, hyphen, underscore)
  - [ ] Check doesn't start with number
  - [ ] Create `validate(&self) -> Result<()>` method on `Label`
- [x] **Implement label merging**
  - [ ] Create `merge_labels(inline: Vec<Label>, annotation: Vec<Label>) -> Vec<Label>`
  - [ ] Combine both sources
  - [ ] Deduplicate (keep first occurrence)
  - [ ] Sort alphabetically for consistency
- [x] **Write tests**
  - [ ] Test inline label parsing (`#tag1 #tag2`)
  - [ ] Test annotation label parsing (`@labels: a, b, c`)
  - [ ] Test normalization (case, whitespace, special chars)
  - [ ] Test validation (length, characters)
  - [ ] Test merging and deduplication
  - [ ] Test edge cases (empty, invalid chars)

**Priority:** HIGH
**Estimate:** 0.5 day
**Dependencies:** Task #2
**Success Criteria:** Can parse and normalize labels; validation works

---

### 6. Create Root Index Model

- [x] **Define `RootIndex` struct in `lash-types/src/index.rs`**
  - [ ] Fields:
    - [ ] `path: PathBuf` - Path to index file
    - [ ] `title: String` - Project title (from H1)
    - [ ] `metadata: IndexMetadata` - Project-level metadata
    - [ ] `entries: Vec<IndexEntry>` - File references
  - [ ] Derive `Debug`, `Clone`
- [x] **Define `IndexMetadata` struct**
  - [ ] Fields:
    - [ ] `project: Option<String>` - Project name
    - [ ] `version: Option<String>` - Version string
    - [ ] `labels: Vec<String>` - Global labels
    - [ ] `custom: HashMap<String, String>` - Other fields
  - [ ] Derive `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
- [x] **Define `IndexEntry` struct**
  - [ ] Fields:
    - [ ] `path: PathBuf` - Relative path to task file
    - [ ] `status: TaskStatus` - Entry status (from checkbox)
    - [ ] `title: Option<String>` - Optional title override
    - [ ] `category: Option<String>` - Optional grouping (from H2 section)
  - [ ] Derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [x] **Implement index validation**
  - [ ] Create `validate(&self, project_root: &Path) -> Result<()>` method
  - [ ] Check all referenced paths exist
  - [ ] Check no duplicate paths
  - [ ] Check paths are within project root
  - [ ] Collect and return all validation errors
- [x] **Implement index traversal**
  - [ ] Create `iter_entries(&self) -> impl Iterator<Item = &IndexEntry>`
  - [ ] Create `get_entry(&self, path: &Path) -> Option<&IndexEntry>`
  - [ ] Create `get_category_entries(&self, category: &str) -> Vec<&IndexEntry>`
- [x] **Add index file detection**
  - [ ] Create `find_index_file(dir: &Path) -> Result<PathBuf>` function
  - [ ] Look for `lash.index.md` first
  - [ ] Fall back to `index.lash.md`
  - [ ] Return error if not found
- [x] **Write tests**
  - [ ] Test validation (existing paths, duplicates)
  - [ ] Test traversal methods
  - [ ] Test file detection
  - [ ] Test with fixture index files
  - [ ] Test error cases

**Priority:** MEDIUM
**Estimate:** 1 day
**Dependencies:** Task #3 (file model)
**Success Criteria:** Can parse and validate root index files; traversal works

---

## Summary

### Total Estimate
**5-7 days** total for core data model

### Completion Criteria
- [x] All tasks above completed
- [x] All core types compile without warnings
- [x] All validation methods work correctly
- [x] Comprehensive test coverage (80%+)
- [x] Types are well-documented with doc comments
- [x] Can build task structures programmatically

### Data Model Diagram

```
TaskFile
├─ path, title, id, hash, mtime
├─ metadata: FileMetadata
│  └─ labels, status, owner, depends_on
└─ tasks: TaskTree
   └─ Vec<Task>
      ├─ id, title, status, depth
      ├─ parent_id, order_index
      └─ metadata: TaskMetadata
         └─ labels, owner, estimate, depends_on

RootIndex
├─ path, title
├─ metadata: IndexMetadata
└─ entries: Vec<IndexEntry>
   └─ path, status, title, category

Dependency
├─ from_task_id (full: file#task)
├─ to_task_id (full: file#task)
└─ kind: DependencyKind
   ├─ Hierarchy (parent->child)
   ├─ ExplicitId (file#task)
   ├─ ExplicitPath (path.md)
   └─ Directory (dir/)
```

### Next Steps
After completing core data model, proceed to:
1. **tasks.markdown-parser.md** - Parse Markdown into these structures
2. **tasks.sqlite-schema.md** - Store these structures in database
