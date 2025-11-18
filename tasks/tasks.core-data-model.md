# Core Data Model Tasks

**Module:** Foundation & Infrastructure
**Priority:** CRITICAL
**Estimated Duration:** 5-7 days
**Dependencies:** tasks.project-setup#4 (error taxonomy)

## Overview

Define the core data structures that represent tasks, files, dependencies, and labels. These types are the foundation of the entire system and must be carefully designed.

**Key Design Principle:** Use arena allocation during parsing, convert to flat indexed representation for database storage (see docs/rust-architecture-recommendations.md).

## Tasks

### 1. Define Task Status Enum

- [ ] **Create `TaskStatus` enum in `lash-types/src/status.rs`**
  - [ ] Variants:
    - [ ] `Open` - Task not yet started
    - [ ] `Done` - Task completed
    - [ ] `Waived` - Task marked as not applicable
    - [ ] `Blocked` - Task cannot proceed (optional extension)
  - [ ] Derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
- [ ] **Implement Display trait**
  - [ ] Format as human-readable: "open", "done", "waived", "blocked"
  - [ ] Use lowercase for consistency
- [ ] **Implement FromStr trait**
  - [ ] Parse from strings: "open", "done", "waived", "blocked"
  - [ ] Case-insensitive parsing
  - [ ] Return error for unknown status strings
- [ ] **Add checkbox character mapping**
  - [ ] Create `to_checkbox_char(&self) -> char` method
    - [ ] `Open` -> `' '` (space)
    - [ ] `Done` -> `'x'`
    - [ ] `Waived` -> `'-'`
    - [ ] `Blocked` -> `'!'`
  - [ ] Create `from_checkbox_char(c: char) -> Result<TaskStatus>` method
  - [ ] Handle both uppercase and lowercase 'X'
- [ ] **Add validation rules**
  - [ ] Create `is_complete(&self) -> bool` method (Done or Waived)
  - [ ] Create `requires_dependencies(&self) -> bool` method
  - [ ] Document semantics in doc comments
- [ ] **Write tests**
  - [ ] Test round-trip: status -> char -> status
  - [ ] Test string parsing (all variants, case-insensitive)
  - [ ] Test display formatting
  - [ ] Test error cases

**Priority:** CRITICAL
**Estimate:** 0.5 day
**Dependencies:** tasks.project-setup#4
**Success Criteria:** Can convert between status and checkbox chars; all tests pass

---

### 2. Implement Task Model

- [ ] **Define `Task` struct in `lash-types/src/task.rs`**
  - [ ] Fields:
    - [ ] `id: String` - Unique ID within file (synthesized if not provided)
    - [ ] `title: String` - Task title/description
    - [ ] `status: TaskStatus` - Current status
    - [ ] `depth: u8` - Nesting level (0 = top-level)
    - [ ] `parent_id: Option<String>` - Parent task ID (if nested)
    - [ ] `order_index: usize` - Position among siblings
    - [ ] `metadata: TaskMetadata` - Optional annotations
    - [ ] `body: Option<String>` - Extended description (optional)
  - [ ] Derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] **Define `TaskMetadata` struct**
  - [ ] Fields:
    - [ ] `labels: Vec<String>` - Inline and explicit labels
    - [ ] `owner: Option<String>` - Assignee
    - [ ] `estimate: Option<String>` - Time estimate (e.g., "2h", "3d")
    - [ ] `depends_on: Vec<DependencyRef>` - Explicit dependencies
    - [ ] `agent_note: Option<String>` - Note for AI agents
    - [ ] `custom: HashMap<String, String>` - Extensibility
  - [ ] Derive `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
- [ ] **Implement `Task` builder pattern**
  - [ ] Create `TaskBuilder` struct
  - [ ] Methods: `new(title)`, `status()`, `depth()`, `parent()`, `label()`, `build()`
  - [ ] Validate depth limit in `build()`
  - [ ] Generate ID if not provided: `task-{index}`
  - [ ] Return errors for invalid configurations
- [ ] **Add validation methods**
  - [ ] `validate(&self, max_depth: u8) -> Result<()>`
    - [ ] Check depth <= max_depth
    - [ ] Check title not empty
    - [ ] Check ID valid (alphanumeric, dash, underscore, colon)
    - [ ] Check order_index >= 0
  - [ ] `is_complete(&self) -> bool` - delegates to status
  - [ ] `is_blocked(&self, deps: &[Dependency]) -> bool` - checks dependencies
- [ ] **Implement hierarchical methods**
  - [ ] `is_child_of(&self, parent_id: &str) -> bool`
  - [ ] `depth_from_parent(&self) -> u8`
- [ ] **Create `TaskTree` struct**
  - [ ] Represent hierarchical task structure
  - [ ] Use `Vec<Task>` internally (flat, indexed)
  - [ ] Maintain parent-child mapping via IDs
  - [ ] Methods:
    - [ ] `add_task(&mut self, task: Task) -> Result<()>`
    - [ ] `get_task(&self, id: &str) -> Option<&Task>`
    - [ ] `get_children(&self, parent_id: &str) -> Vec<&Task>`
    - [ ] `get_descendants(&self, id: &str) -> Vec<&Task>` (recursive)
    - [ ] `validate(&self, max_depth: u8) -> Result<()>`
- [ ] **Write tests**
  - [ ] Test task creation via builder
  - [ ] Test validation (depth limits, empty titles)
  - [ ] Test parent-child relationships
  - [ ] Test TaskTree operations (add, get, children)
  - [ ] Test edge cases (cycles, missing parents)

**Priority:** CRITICAL
**Estimate:** 2 days
**Dependencies:** Task #1
**Success Criteria:** Can build task trees programmatically; validation works; all tests pass

---

### 3. Implement File Metadata Model

- [ ] **Define `TaskFile` struct in `lash-types/src/file.rs`**
  - [ ] Fields:
    - [ ] `path: PathBuf` - Relative path from project root
    - [ ] `title: String` - H1 title from file
    - [ ] `id: String` - File identifier (from `@id` or derived from path)
    - [ ] `metadata: FileMetadata` - File-level annotations
    - [ ] `tasks: TaskTree` - Hierarchical task structure
    - [ ] `hash: String` - Content hash (blake3)
    - [ ] `mtime: SystemTime` - Last modified time
  - [ ] Derive `Debug`, `Clone`
- [ ] **Define `FileMetadata` struct**
  - [ ] Fields:
    - [ ] `labels: Vec<String>` - File-level labels
    - [ ] `status: Option<String>` - Overall status
    - [ ] `owner: Option<String>` - File owner
    - [ ] `created: Option<String>` - Creation date (YYYY-MM-DD)
    - [ ] `depends_on: Vec<DependencyRef>` - File-level dependencies
    - [ ] `custom: HashMap<String, String>` - Other annotations
  - [ ] Derive `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
- [ ] **Implement content hashing**
  - [ ] Add `blake3` dependency to `lash-types`
  - [ ] Create `compute_hash(content: &str) -> String` function
  - [ ] Hash the full file content
  - [ ] Return hex-encoded hash string
  - [ ] Create `hash_matches(&self, content: &str) -> bool` method
- [ ] **Implement file-level validation**
  - [ ] Create `validate(&self, config: &LashConfig) -> Result<()>` method
  - [ ] Check file ID is unique (within project context, enforced elsewhere)
  - [ ] Validate all tasks in tree
  - [ ] Check for task ID uniqueness within file
  - [ ] Validate dependency references (syntax only, not resolution)
- [ ] **Implement file status computation**
  - [ ] Create `compute_status(&self) -> FileStatus` method
    - [ ] `Complete` if all top-level tasks complete
    - [ ] `InProgress` if any top-level task in progress
    - [ ] `Blocked` if any top-level task blocked
    - [ ] `Empty` if no tasks
  - [ ] Define `FileStatus` enum (Complete, InProgress, Blocked, Empty)
- [ ] **Add ID synthesis**
  - [ ] Create `synthesize_file_id(path: &Path) -> String` function
  - [ ] Use dot-delimited path: `core.api.auth` from `core/api/auth.md`
  - [ ] Strip `.md` extension
  - [ ] Convert `/` to `.`
  - [ ] Normalize to lowercase
- [ ] **Write tests**
  - [ ] Test hash computation and comparison
  - [ ] Test ID synthesis from paths
  - [ ] Test status computation
  - [ ] Test validation (unique IDs, etc.)
  - [ ] Test round-trip serialization

**Priority:** CRITICAL
**Estimate:** 1 day
**Dependencies:** Task #2
**Success Criteria:** Can represent complete task file in memory; hash/status computation works

---

### 4. Define Dependency Types

- [ ] **Create `DependencyKind` enum in `lash-types/src/dependency.rs`**
  - [ ] Variants:
    - [ ] `Hierarchy` - Parent depends on child (implicit from nesting)
    - [ ] `ExplicitId` - `@depends-on: file-id#task-id`
    - [ ] `ExplicitPath` - `@depends-on: path/to/file.md`
    - [ ] `Directory` - File depends on subdirectory completion
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
- [ ] **Define `DependencyRef` struct**
  - [ ] Fields:
    - [ ] `target: String` - Raw reference string
    - [ ] `kind: DependencyKind` - Parsed kind
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`
- [ ] **Define `Dependency` struct (resolved)**
  - [ ] Fields:
    - [ ] `from_task_id: String` - Source task full ID (file-id#task-id)
    - [ ] `to_task_id: String` - Target task full ID
    - [ ] `kind: DependencyKind` - Dependency type
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
- [ ] **Implement parsing for `@depends-on` references**
  - [ ] Create `parse_dependency_ref(s: &str) -> Result<DependencyRef>` function
  - [ ] Detect format:
    - [ ] File path: ends with `.md`
    - [ ] Full ID: contains `#` (split on first `#`)
    - [ ] Relative path: `../file.md` or `./file.md`
  - [ ] Store raw string and parsed kind
  - [ ] Validate syntax (not resolution)
- [ ] **Create full task ID helpers**
  - [ ] Create `make_full_id(file_id: &str, task_id: &str) -> String`
  - [ ] Format: `{file_id}#{task_id}`
  - [ ] Create `parse_full_id(full_id: &str) -> Result<(String, String)>`
  - [ ] Split on first `#`
  - [ ] Return error if no `#` found
- [ ] **Add validation**
  - [ ] Validate dependency reference syntax
  - [ ] Check for empty strings
  - [ ] Check for valid characters in IDs
  - [ ] Validate path format
- [ ] **Add display formatting**
  - [ ] Implement `Display` for `DependencyRef`
  - [ ] Format based on kind:
    - [ ] `ExplicitId`: `file-id#task-id`
    - [ ] `ExplicitPath`: `path/to/file.md`
    - [ ] `Hierarchy`: `(implicit)`
    - [ ] `Directory`: `dir:path/to/dir/`
- [ ] **Write tests**
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

- [ ] **Define `Label` struct in `lash-types/src/label.rs`**
  - [ ] Fields:
    - [ ] `name: String` - Normalized label name
  - [ ] Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`
- [ ] **Implement label parsing**
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
- [ ] **Implement label normalization**
  - [ ] Create `normalize(s: &str) -> String` function
  - [ ] Convert to lowercase
  - [ ] Trim whitespace
  - [ ] Replace spaces with hyphens
  - [ ] Keep alphanumeric, hyphen, underscore only
  - [ ] Strip leading/trailing hyphens
- [ ] **Implement validation**
  - [ ] Create `is_valid_label(s: &str) -> bool` function
  - [ ] Check length (1-50 characters)
  - [ ] Check allowed characters (alphanumeric, hyphen, underscore)
  - [ ] Check doesn't start with number
  - [ ] Create `validate(&self) -> Result<()>` method on `Label`
- [ ] **Implement label merging**
  - [ ] Create `merge_labels(inline: Vec<Label>, annotation: Vec<Label>) -> Vec<Label>`
  - [ ] Combine both sources
  - [ ] Deduplicate (keep first occurrence)
  - [ ] Sort alphabetically for consistency
- [ ] **Write tests**
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

- [ ] **Define `RootIndex` struct in `lash-types/src/index.rs`**
  - [ ] Fields:
    - [ ] `path: PathBuf` - Path to index file
    - [ ] `title: String` - Project title (from H1)
    - [ ] `metadata: IndexMetadata` - Project-level metadata
    - [ ] `entries: Vec<IndexEntry>` - File references
  - [ ] Derive `Debug`, `Clone`
- [ ] **Define `IndexMetadata` struct**
  - [ ] Fields:
    - [ ] `project: Option<String>` - Project name
    - [ ] `version: Option<String>` - Version string
    - [ ] `labels: Vec<String>` - Global labels
    - [ ] `custom: HashMap<String, String>` - Other fields
  - [ ] Derive `Debug`, `Clone`, `Default`, `Serialize`, `Deserialize`
- [ ] **Define `IndexEntry` struct**
  - [ ] Fields:
    - [ ] `path: PathBuf` - Relative path to task file
    - [ ] `status: TaskStatus` - Entry status (from checkbox)
    - [ ] `title: Option<String>` - Optional title override
    - [ ] `category: Option<String>` - Optional grouping (from H2 section)
  - [ ] Derive `Debug`, `Clone`, `Serialize`, `Deserialize`
- [ ] **Implement index validation**
  - [ ] Create `validate(&self, project_root: &Path) -> Result<()>` method
  - [ ] Check all referenced paths exist
  - [ ] Check no duplicate paths
  - [ ] Check paths are within project root
  - [ ] Collect and return all validation errors
- [ ] **Implement index traversal**
  - [ ] Create `iter_entries(&self) -> impl Iterator<Item = &IndexEntry>`
  - [ ] Create `get_entry(&self, path: &Path) -> Option<&IndexEntry>`
  - [ ] Create `get_category_entries(&self, category: &str) -> Vec<&IndexEntry>`
- [ ] **Add index file detection**
  - [ ] Create `find_index_file(dir: &Path) -> Result<PathBuf>` function
  - [ ] Look for `lash.index.md` first
  - [ ] Fall back to `index.lash.md`
  - [ ] Return error if not found
- [ ] **Write tests**
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
- [ ] All tasks above completed
- [ ] All core types compile without warnings
- [ ] All validation methods work correctly
- [ ] Comprehensive test coverage (80%+)
- [ ] Types are well-documented with doc comments
- [ ] Can build task structures programmatically

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
