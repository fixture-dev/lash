//! Detailed error explanations for the `lash explain` command
//!
//! This module provides comprehensive documentation for each error code,
//! including:
//! - What the error means
//! - Why it occurs
//! - How to fix it
//! - Examples of the error and correct code

use crate::error::codes;

/// Detailed explanation of an error code
#[derive(Debug, Clone)]
pub struct ErrorExplanation {
    /// The error code being explained
    pub code: &'static str,

    /// One-line summary of the error
    pub summary: &'static str,

    /// Detailed description of what causes this error
    pub description: &'static str,

    /// Why this error matters (what could go wrong if not fixed)
    pub why_it_matters: &'static str,

    /// How to fix the error
    pub how_to_fix: &'static str,

    /// Example of code that would produce this error
    pub example_bad: Option<&'static str>,

    /// Example of correct code
    pub example_good: Option<&'static str>,
}

impl ErrorExplanation {
    /// Format the explanation as markdown text
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("# Error: {}\n\n", self.code));
        output.push_str(&format!("## {}\n\n", self.summary));
        output.push_str(&format!("**Description:** {}\n\n", self.description));
        output.push_str(&format!("**Why it matters:** {}\n\n", self.why_it_matters));
        output.push_str(&format!("**How to fix:** {}\n\n", self.how_to_fix));

        if let Some(bad) = self.example_bad {
            output.push_str("### Example (Incorrect)\n\n");
            output.push_str("```markdown\n");
            output.push_str(bad);
            output.push_str("\n```\n\n");
        }

        if let Some(good) = self.example_good {
            output.push_str("### Example (Correct)\n\n");
            output.push_str("```markdown\n");
            output.push_str(good);
            output.push_str("\n```\n\n");
        }

        output
    }
}

/// Get the explanation for a specific error code
///
/// Returns `None` if the error code is not recognized.
#[must_use]
pub fn explain_error(code: &str) -> Option<ErrorExplanation> {
    match code {
        // ===== Parse Errors =====
        codes::E_PARSE_INVALID_CHECKBOX => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_CHECKBOX,
            summary: "Invalid checkbox syntax",
            description: "The checkbox marker in your task is not valid. Lash only recognizes four checkbox states: [ ] for open, [x] for done, [-] for waived, and [!] for blocked.",
            why_it_matters: "Invalid checkboxes prevent Lash from parsing your task files correctly, which means the task won't be indexed, tracked, or included in dependency resolution.",
            how_to_fix: "Replace the invalid checkbox with one of the valid formats: [ ], [x], [-], or [!]. Run `lash format` to automatically fix checkbox formatting.",
            example_bad: Some("- [*] Invalid checkbox\n- [v] Also invalid\n- [] Missing space"),
            example_good: Some("- [ ] Open task\n- [x] Completed task\n- [-] Waived task\n- [!] Blocked task"),
        }),

        codes::E_PARSE_INVALID_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_ANNOTATION,
            summary: "Malformed annotation",
            description: "An annotation in your task file doesn't follow the required @key: value format. Annotations must start with @, followed by the annotation name, a colon, and the value.",
            why_it_matters: "Malformed annotations can't be parsed, so Lash won't be able to read metadata like task IDs, labels, owners, or dependencies.",
            how_to_fix: "Ensure annotations follow the format: @key: value. There must be a space after the colon. Run `lash format` to normalize formatting.",
            example_bad: Some("@id task-1\n@labels: frontend, backend\n@owner:Alice"),
            example_good: Some("@id: task-1\n@labels: frontend, backend\n@owner: Alice"),
        }),

        codes::E_PARSE_INVALID_HEADER => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_HEADER,
            summary: "Invalid header format",
            description: "A header in your file doesn't follow proper Markdown syntax. Headers must start with one or more # symbols followed by a space.",
            why_it_matters: "Invalid headers break the document structure, making it impossible for Lash to organize tasks into sections.",
            how_to_fix: "Add a space after the # symbols in your headers. For example, change '##Tasks' to '## Tasks'.",
            example_bad: Some("##Tasks\n###Section 1"),
            example_good: Some("## Tasks\n### Section 1"),
        }),

        codes::E_PARSE_UNEXPECTED_DEPTH => Some(ErrorExplanation {
            code: codes::E_PARSE_UNEXPECTED_DEPTH,
            summary: "Unexpected indentation depth",
            description: "A task or subtask has incorrect indentation. Each level of nesting should be indented by exactly 2 spaces.",
            why_it_matters: "Incorrect indentation breaks the task hierarchy, which affects dependency resolution and task organization.",
            how_to_fix: "Adjust the indentation to use exactly 2 spaces per nesting level. Run `lash format` to automatically fix indentation.",
            example_bad: Some("- [ ] Parent\n   - [ ] Child (3 spaces)\n- [ ] Another (wrong depth)"),
            example_good: Some("- [ ] Parent\n  - [ ] Child (2 spaces)\n    - [ ] Grandchild (4 spaces)"),
        }),

        codes::E_PARSE_INVALID_DATE => Some(ErrorExplanation {
            code: codes::E_PARSE_INVALID_DATE,
            summary: "Invalid date format",
            description: "A date annotation doesn't use the required YYYY-MM-DD format.",
            why_it_matters: "Invalid dates can't be parsed or compared, breaking features like task filtering by date and timeline calculations.",
            how_to_fix: "Change the date to YYYY-MM-DD format. For example, '2024-01-15' for January 15, 2024.",
            example_bad: Some("@created: 01/15/2024\n@created: Jan 15, 2024"),
            example_good: Some("@created: 2024-01-15"),
        }),

        // ===== Lint Errors =====
        codes::E_LINT_DUPLICATE_ID => Some(ErrorExplanation {
            code: codes::E_LINT_DUPLICATE_ID,
            summary: "Duplicate task ID",
            description: "Two or more tasks in the same file have the same @id annotation. Task IDs must be unique within a file.",
            why_it_matters: "Duplicate IDs make it impossible to reference specific tasks unambiguously, breaking dependency links and task lookups.",
            how_to_fix: "Rename one of the duplicate IDs to a unique value. Choose descriptive, unique identifiers for each task.",
            example_bad: Some("- [ ] First task\n  @id: setup\n\n- [ ] Second task\n  @id: setup"),
            example_good: Some("- [ ] First task\n  @id: setup-database\n\n- [ ] Second task\n  @id: setup-server"),
        }),

        codes::E_LINT_UNKNOWN_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_LINT_UNKNOWN_ANNOTATION,
            summary: "Unknown annotation",
            description: "The annotation used is not recognized by Lash. Valid annotations are: @id, @labels, @owner, @estimate, @depends-on, @created, @doc, and @agent-note.",
            why_it_matters: "Unknown annotations are ignored by Lash, which may indicate a typo or misunderstanding of the annotation system.",
            how_to_fix: "Check the annotation name for typos, or remove it if it's not needed. Refer to the documentation for the list of valid annotations.",
            example_bad: Some("@task-id: my-task\n@priority: high"),
            example_good: Some("@id: my-task\n@labels: priority-high"),
        }),

        codes::E_LINT_DEPTH_EXCEEDED => Some(ErrorExplanation {
            code: codes::E_LINT_DEPTH_EXCEEDED,
            summary: "Task nesting exceeds maximum depth",
            description: "Tasks are nested too deeply. The maximum recommended depth is 4 levels (parent, child, grandchild, great-grandchild).",
            why_it_matters: "Excessive nesting makes task files hard to read and maintain. It often indicates that tasks should be broken into separate files.",
            how_to_fix: "Flatten the task hierarchy by moving deeply nested tasks to a separate file or restructuring the task breakdown.",
            example_bad: Some("- [ ] Level 1\n  - [ ] Level 2\n    - [ ] Level 3\n      - [ ] Level 4\n        - [ ] Level 5 (too deep)"),
            example_good: Some("- [ ] High-level task\n  @depends-on: detailed-tasks.md#task:setup\n\n(Move details to detailed-tasks.md)"),
        }),

        codes::E_LINT_STATUS_INCONSISTENCY => Some(ErrorExplanation {
            code: codes::E_LINT_STATUS_INCONSISTENCY,
            summary: "Parent task marked done with incomplete children",
            description: "A parent task is marked as done [x] but has child tasks that are still open [ ].",
            why_it_matters: "This creates logical inconsistency in your task hierarchy. A parent can only be complete when all its children are complete or waived.",
            how_to_fix: "Either mark all child tasks as done/waived, or change the parent status to open or blocked.",
            example_bad: Some("- [x] Complete feature\n  - [ ] Write code\n  - [ ] Write tests"),
            example_good: Some("- [ ] Complete feature\n  - [x] Write code\n  - [x] Write tests\n\nOR\n\n- [x] Complete feature\n  - [x] Write code\n  - [x] Write tests"),
        }),

        codes::E_LINT_INVALID_LABEL => Some(ErrorExplanation {
            code: codes::E_LINT_INVALID_LABEL,
            summary: "Invalid label format",
            description: "A label contains invalid characters. Labels must be alphanumeric with hyphens, and multiple labels should be comma-separated.",
            why_it_matters: "Invalid labels can't be used for filtering and may cause parsing errors.",
            how_to_fix: "Use only letters, numbers, and hyphens in labels. Separate multiple labels with commas.",
            example_bad: Some("@labels: front-end!, back_end\n@labels: high priority"),
            example_good: Some("@labels: front-end, back-end\n@labels: high-priority"),
        }),

        codes::E_LINT_MISSING_ANNOTATION => Some(ErrorExplanation {
            code: codes::E_LINT_MISSING_ANNOTATION,
            summary: "Missing required annotation",
            description: "A task is missing a required annotation, typically @id. Some projects require certain annotations for proper tracking.",
            why_it_matters: "Missing required annotations prevent proper task identification and cross-referencing.",
            how_to_fix: "Add the required annotation to the task. For @id, choose a unique, descriptive identifier.",
            example_bad: Some("- [ ] My task without ID"),
            example_good: Some("- [ ] My task\n  @id: my-task-id"),
        }),

        codes::E_LINT_BAD_INDENTATION => Some(ErrorExplanation {
            code: codes::E_LINT_BAD_INDENTATION,
            summary: "Incorrect indentation",
            description: "The indentation doesn't match the expected 2-space increments for task nesting.",
            why_it_matters: "Inconsistent indentation breaks the task hierarchy and makes files hard to read.",
            how_to_fix: "Run `lash format` to automatically fix indentation to the standard 2-space increments.",
            example_bad: Some("- [ ] Parent\n   - [ ] Child (3 spaces)\n    - [ ] Another (4 spaces)"),
            example_good: Some("- [ ] Parent\n  - [ ] Child (2 spaces)\n  - [ ] Another (2 spaces)"),
        }),

        codes::W_SEM_DOC_FRAGMENT => Some(ErrorExplanation {
            code: codes::W_SEM_DOC_FRAGMENT,
            summary: "@doc: fragment does not match any heading",
            description: "An @doc annotation references a #fragment that does not exist in the target document. Lash matches fragments against headings using case- and punctuation-insensitive normalization: both the fragment and each heading are lowercased, '-' is treated as whitespace, every non-alphanumeric/non-whitespace character (including '<', '>', '/', '.', '_', '(', ')', and backticks) is stripped *without* introducing a hyphen boundary, and runs of whitespace are collapsed. Two strings match when they reduce to the same canonical form.",
            why_it_matters: "Broken @doc: fragments mean readers (humans and agents) following the link cannot land on the intended section. The lint catches them so they fail loudly instead of silently 404-ing in a renderer that ignores anchors.",
            how_to_fix: "Open the target document, find the heading you want, and write the fragment so it normalizes to the same canonical form. The warning's help text lists existing headings in the target — pick one. Convention: lowercase the heading, replace spaces with '-', and drop punctuation entirely (do not turn '/' or '.' into a hyphen).",
            example_bad: Some("# Heading: Pack manifest (`<pack>/SKILL.md`)\n@doc: ../docs/skills.md#pack-manifest-pack-skill-md\n# (`<` `>` `/` are stripped without producing a boundary, so this slug is wrong)"),
            example_good: Some("# Heading: Pack manifest (`<pack>/SKILL.md`)\n@doc: ../docs/skills.md#pack-manifest-packskillmd\n\n# Heading: Validation rules (must pass at index time)\n@doc: ../docs/skills.md#validation-rules-must-pass-at-index-time"),
        }),

        // ===== Dependency Errors =====
        codes::E_DEP_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_DEP_NOT_FOUND,
            summary: "Dependency target not found",
            description: "A task references a dependency that doesn't exist. The referenced file or task ID could not be found.",
            why_it_matters: "Broken dependencies prevent accurate dependency graph construction and can cause tasks to appear blocked incorrectly.",
            how_to_fix: "Check that the file path and task ID in the @depends-on annotation are correct. Verify the referenced task exists and has the correct @id.",
            example_bad: Some("@depends-on: path/to/missing.md#task:nonexistent-id"),
            example_good: Some("@depends-on: path/to/existing.md#task:valid-id"),
        }),

        codes::E_DEP_CYCLE => Some(ErrorExplanation {
            code: codes::E_DEP_CYCLE,
            summary: "Circular dependency detected",
            description: "A cycle exists in the dependency graph where task A depends on B, which depends on C, which depends back on A.",
            why_it_matters: "Circular dependencies create logical impossibilities and prevent proper task ordering. No task in the cycle can ever be started.",
            how_to_fix: "Break the cycle by removing one of the dependencies or restructuring the task relationships. The error message shows the cycle path.",
            example_bad: Some("Task A @depends-on: B\nTask B @depends-on: C\nTask C @depends-on: A"),
            example_good: Some("Task A @depends-on: B\nTask B @depends-on: C\nTask C has no dependencies"),
        }),

        codes::E_DEP_INVALID_REF => Some(ErrorExplanation {
            code: codes::E_DEP_INVALID_REF,
            summary: "Invalid dependency reference format",
            description: "The @depends-on annotation doesn't follow the required format: path/to/file.md#task:id",
            why_it_matters: "Invalid reference format prevents Lash from resolving dependencies correctly.",
            how_to_fix: "Use the format: path/to/file.md#task:id where the path is relative to the project root.",
            example_bad: Some("@depends-on: file.md#id\n@depends-on: just-an-id"),
            example_good: Some("@depends-on: tasks/setup.md#task:database-setup"),
        }),

        // ===== Index Errors =====
        codes::E_INDEX_CORRUPTED => Some(ErrorExplanation {
            code: codes::E_INDEX_CORRUPTED,
            summary: "Database corruption detected",
            description: "The SQLite database has become corrupted or contains invalid data.",
            why_it_matters: "A corrupted index prevents Lash from functioning correctly and may lead to data loss or incorrect results.",
            how_to_fix: "Run `lash index --rebuild` to rebuild the database from scratch from your Markdown files.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_INDEX_VERSION_MISMATCH => Some(ErrorExplanation {
            code: codes::E_INDEX_VERSION_MISMATCH,
            summary: "Database schema version mismatch",
            description: "The database was created with a different version of Lash and needs to be migrated to the current schema.",
            why_it_matters: "Version mismatches can cause incorrect behavior or crashes when Lash tries to read incompatible database structures.",
            how_to_fix: "Run `lash index --migrate` to update the database schema, or `lash index --rebuild` to rebuild from scratch.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_INDEX_OUT_OF_SYNC => Some(ErrorExplanation {
            code: codes::E_INDEX_OUT_OF_SYNC,
            summary: "Index is out of sync with files",
            description: "The SQLite index doesn't match the current state of your Markdown files. Files have been modified since the last index update.",
            why_it_matters: "An out-of-sync index means queries may return stale or incorrect data.",
            how_to_fix: "Run `lash index` to update the index. Lash automatically detects changed files and updates only what's necessary.",
            example_bad: None,
            example_good: None,
        }),

        // ===== Query Errors =====
        codes::E_QUERY_INVALID_SYNTAX => Some(ErrorExplanation {
            code: codes::E_QUERY_INVALID_SYNTAX,
            summary: "Invalid query syntax",
            description: "The search query uses invalid syntax that can't be parsed.",
            why_it_matters: "Invalid query syntax prevents the search from executing.",
            how_to_fix: "Check the query syntax. Run `lash help search` for documentation on search query syntax.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_QUERY_NO_RESULTS => Some(ErrorExplanation {
            code: codes::E_QUERY_NO_RESULTS,
            summary: "No results found",
            description: "The query executed successfully but returned no matching tasks.",
            why_it_matters: "This may indicate the search criteria are too restrictive or the expected tasks don't exist.",
            how_to_fix: "Try broadening your search criteria, removing some filters, or checking that the tasks you're looking for actually exist.",
            example_bad: None,
            example_good: None,
        }),

        // ===== Config Errors =====
        codes::E_CONFIG_ROOT_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_CONFIG_ROOT_NOT_FOUND,
            summary: "Project root not found",
            description: "Lash couldn't find a project root directory. It looks for lash.index.md, index.lash.md, or a .lash/ directory.",
            why_it_matters: "Without a project root, Lash doesn't know where to look for task files or store the database.",
            how_to_fix: "Run `lash init` to create a new project, or navigate to an existing Lash project directory. You can also use --root to specify the project root explicitly.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_CONFIG_INVALID_VALUE => Some(ErrorExplanation {
            code: codes::E_CONFIG_INVALID_VALUE,
            summary: "Invalid configuration value",
            description: "A configuration value is invalid or out of acceptable range.",
            why_it_matters: "Invalid configuration prevents Lash from starting or causes incorrect behavior.",
            how_to_fix: "Check the configuration file for the invalid value and correct it according to the documentation.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_CONFIG_PARSE_ERROR => Some(ErrorExplanation {
            code: codes::E_CONFIG_PARSE_ERROR,
            summary: "Configuration parse error",
            description: "The configuration file couldn't be parsed as valid TOML.",
            why_it_matters: "A malformed configuration file prevents Lash from loading user preferences.",
            how_to_fix: "Check the configuration file for syntax errors. Ensure it's valid TOML format.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_CONFIG_MISSING_INDEX => Some(ErrorExplanation {
            code: codes::E_CONFIG_MISSING_INDEX,
            summary: "Index file not found",
            description: "The project root exists but doesn't contain an index file (lash.index.md or index.lash.md).",
            why_it_matters: "The index file is the entry point for task navigation and defines the project structure.",
            how_to_fix: "Create an index file at the project root: either lash.index.md or index.lash.md.",
            example_bad: None,
            example_good: None,
        }),

        // ===== IO Errors =====
        codes::E_IO_FILE_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_IO_FILE_NOT_FOUND,
            summary: "File not found",
            description: "The specified file doesn't exist.",
            why_it_matters: "Lash can't operate on files that don't exist.",
            how_to_fix: "Check that the file path is correct. If the file was moved or deleted, update any references to it.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_IO_READ_ERROR => Some(ErrorExplanation {
            code: codes::E_IO_READ_ERROR,
            summary: "Failed to read file",
            description: "An I/O error occurred while trying to read a file.",
            why_it_matters: "If Lash can't read files, it can't parse tasks or update the index.",
            how_to_fix: "Check file permissions, disk space, and that the file isn't locked by another process.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_IO_WRITE_ERROR => Some(ErrorExplanation {
            code: codes::E_IO_WRITE_ERROR,
            summary: "Failed to write file",
            description: "An I/O error occurred while trying to write to a file.",
            why_it_matters: "Write errors prevent Lash from saving changes, formatting files, or creating new files.",
            how_to_fix: "Check that you have write permissions, sufficient disk space, and that the file isn't read-only.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_IO_PERMISSION_DENIED => Some(ErrorExplanation {
            code: codes::E_IO_PERMISSION_DENIED,
            summary: "Permission denied",
            description: "You don't have permission to access the specified file or directory.",
            why_it_matters: "Permission errors prevent Lash from reading or writing files.",
            how_to_fix: "Check file permissions and adjust them if needed, or run Lash with appropriate permissions.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_IO_INVALID_PATH => Some(ErrorExplanation {
            code: codes::E_IO_INVALID_PATH,
            summary: "Invalid path",
            description: "The specified path is invalid or contains illegal characters.",
            why_it_matters: "Invalid paths can't be used to access files.",
            how_to_fix: "Check that the path is properly formatted and doesn't contain invalid characters for your operating system.",
            example_bad: None,
            example_good: None,
        }),

        // ===== Internal Errors =====
        codes::E_INTERNAL => Some(ErrorExplanation {
            code: codes::E_INTERNAL,
            summary: "Internal error",
            description: "An unexpected internal error occurred. This is likely a bug in Lash.",
            why_it_matters: "Internal errors indicate bugs that should be reported to the Lash developers.",
            how_to_fix: "Please report this error as a bug on the Lash issue tracker, including the full error message and steps to reproduce.",
            example_bad: None,
            example_good: None,
        }),

        // ===== Task Creation Errors =====
        codes::E_CREATE_EMPTY_TITLE => Some(ErrorExplanation {
            code: codes::E_CREATE_EMPTY_TITLE,
            summary: "Task title is empty",
            description: "The task title provided is empty or contains only whitespace. Every task requires a meaningful title to identify it.",
            why_it_matters: "Tasks without titles cannot be identified or tracked. The title is the primary way users and agents reference tasks.",
            how_to_fix: "Provide a non-empty, descriptive title for the task. A good title clearly describes what needs to be done.",
            example_bad: Some("lash add \"\"\nlash add \"   \""),
            example_good: Some("lash add \"Implement user authentication\"\nlash add \"Fix login page CSS\""),
        }),

        codes::E_CREATE_TITLE_TOO_LONG => Some(ErrorExplanation {
            code: codes::E_CREATE_TITLE_TOO_LONG,
            summary: "Task title exceeds maximum length",
            description: "The task title is longer than the maximum allowed length of 200 characters. Long titles become unwieldy in displays and reports.",
            why_it_matters: "Excessively long titles make task lists hard to read and can cause display issues in the TUI and other interfaces.",
            how_to_fix: "Shorten the title to 200 characters or fewer. Move detailed information to the task description or agent note instead.",
            example_bad: Some("lash add \"This is an extremely long task title that goes into way too much detail about what needs to be done when really it should be a short summary...\""),
            example_good: Some("lash add \"Implement caching layer\" --agent-note \"Consider Redis or memcached for distributed caching\""),
        }),

        codes::E_CREATE_FILE_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_CREATE_FILE_NOT_FOUND,
            summary: "Target file does not exist",
            description: "The file specified with --file does not exist. When using an explicit file path, the file must exist unless you're creating a new file.",
            why_it_matters: "Tasks can only be added to existing, valid Lash task files. Creating tasks in non-existent files would fail.",
            how_to_fix: "Either create the file first, or use --file with --file-title to automatically create a new task file.",
            example_bad: Some("lash add \"My task\" --file nonexistent.md"),
            example_good: Some("lash add \"My task\" --file tasks/new-feature.md --file-title \"New Feature Tasks\""),
        }),

        codes::E_CREATE_FILE_NOT_WRITABLE => Some(ErrorExplanation {
            code: codes::E_CREATE_FILE_NOT_WRITABLE,
            summary: "Target file is not writable",
            description: "The target file exists but cannot be written to. This usually means the file is read-only or you don't have write permissions.",
            why_it_matters: "Lash needs write access to add tasks to a file. Without it, the task cannot be saved.",
            how_to_fix: "Check the file permissions and ensure you have write access. On Unix systems, use chmod to add write permissions if needed.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_CREATE_FILE_PARSE_FAILED => Some(ErrorExplanation {
            code: codes::E_CREATE_FILE_PARSE_FAILED,
            summary: "Target file failed to parse",
            description: "The target file exists but contains invalid syntax that Lash cannot parse. The file may be corrupted or not follow the expected format.",
            why_it_matters: "Lash must understand the file structure to safely add a task without breaking existing content.",
            how_to_fix: "Run `lash lint <file>` to see the parsing errors and fix them before adding new tasks.",
            example_bad: None,
            example_good: None,
        }),

        codes::E_CREATE_PARENT_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_CREATE_PARENT_NOT_FOUND,
            summary: "Parent task not found",
            description: "The parent task specified with --parent does not exist in the target file. Subtasks must have valid parent tasks.",
            why_it_matters: "Creating a subtask requires a valid parent. Without one, the task hierarchy would be broken.",
            how_to_fix: "Ensure the parent task ID exists in the file, or omit --parent to create a top-level task.",
            example_bad: Some("lash add \"Subtask\" --parent nonexistent-parent"),
            example_good: Some("lash add \"Subtask\" --parent existing-task-id\nlash add \"Top-level task\"  # No parent needed"),
        }),

        codes::E_CREATE_DEPTH_LIMIT_EXCEEDED => Some(ErrorExplanation {
            code: codes::E_CREATE_DEPTH_LIMIT_EXCEEDED,
            summary: "Task would exceed maximum nesting depth",
            description: "Creating this task as a subtask of the specified parent would exceed the maximum nesting depth (default: 3 levels).",
            why_it_matters: "Excessive nesting makes task files hard to read and manage. It often indicates tasks should be reorganized or split into separate files.",
            how_to_fix: "Choose a parent task at a shallower depth, create a top-level task instead, or reorganize tasks into separate files.",
            example_bad: Some("# Already at depth 3:\n- [ ] Level 1\n  - [ ] Level 2\n    - [ ] Level 3\n      # Cannot add here"),
            example_good: Some("# Create at shallower level or as top-level:\nlash add \"New task\"  # Top-level\nlash add \"New task\" --parent level-1-task"),
        }),

        codes::E_CREATE_DUPLICATE_ID => Some(ErrorExplanation {
            code: codes::E_CREATE_DUPLICATE_ID,
            summary: "Task ID is already in use",
            description: "The task ID specified with --id is already used by another task in the same file. Task IDs must be unique within each file.",
            why_it_matters: "Duplicate IDs make it impossible to uniquely reference tasks, breaking dependencies and cross-references.",
            how_to_fix: "Choose a different, unique ID for the task, or omit --id to let Lash auto-generate one from the title.",
            example_bad: Some("# If 'setup' already exists:\nlash add \"Another task\" --id setup"),
            example_good: Some("lash add \"Another task\" --id setup-phase-2\nlash add \"Another task\"  # Auto-generates ID"),
        }),

        codes::E_CREATE_INVALID_ID_FORMAT => Some(ErrorExplanation {
            code: codes::E_CREATE_INVALID_ID_FORMAT,
            summary: "Task ID format is invalid",
            description: "The task ID contains invalid characters. IDs must contain only alphanumeric characters, hyphens, underscores, and colons.",
            why_it_matters: "Valid IDs are necessary for reliable cross-referencing and URL-safe task references.",
            how_to_fix: "Use only letters, numbers, hyphens (-), underscores (_), and colons (:) in task IDs.",
            example_bad: Some("lash add \"Task\" --id \"my task!\"\nlash add \"Task\" --id \"task with spaces\""),
            example_good: Some("lash add \"Task\" --id my-task\nlash add \"Task\" --id task_v2\nlash add \"Task\" --id module:feature"),
        }),

        codes::E_CREATE_INVALID_LABEL => Some(ErrorExplanation {
            code: codes::E_CREATE_INVALID_LABEL,
            summary: "Label format is invalid",
            description: "One or more labels contain invalid characters. Labels must be alphanumeric with hyphens only, no spaces or special characters.",
            why_it_matters: "Consistent label formatting ensures reliable filtering and searching across tasks.",
            how_to_fix: "Use only letters, numbers, and hyphens in labels. Replace spaces with hyphens.",
            example_bad: Some("lash add \"Task\" --label \"my label\"\nlash add \"Task\" --label \"urgent!\""),
            example_good: Some("lash add \"Task\" --label my-label\nlash add \"Task\" --label urgent --label backend"),
        }),

        codes::E_CREATE_INVALID_ESTIMATE => Some(ErrorExplanation {
            code: codes::E_CREATE_INVALID_ESTIMATE,
            summary: "Time estimate format is invalid",
            description: "The time estimate doesn't follow a recognized format. Estimates should use units like minutes (m), hours (h), days (d), or weeks (w).",
            why_it_matters: "Valid time estimates enable sprint planning and progress tracking features.",
            how_to_fix: "Use formats like: 30m (minutes), 2h (hours), 1d (days), 1w (weeks), or combined: 2d 4h.",
            example_bad: Some("lash add \"Task\" --estimate \"two hours\"\nlash add \"Task\" --estimate \"long\""),
            example_good: Some("lash add \"Task\" --estimate 2h\nlash add \"Task\" --estimate 1d\nlash add \"Task\" --estimate \"2d 4h\""),
        }),

        codes::E_CREATE_DEPENDENCY_NOT_FOUND => Some(ErrorExplanation {
            code: codes::E_CREATE_DEPENDENCY_NOT_FOUND,
            summary: "Dependency reference not found",
            description: "A dependency specified with --depends-on references a task that doesn't exist or cannot be found.",
            why_it_matters: "Dependencies must point to valid tasks. Broken dependencies cause incorrect blocking status.",
            how_to_fix: "Verify the referenced task exists. Use format: path/to/file.md#task:id for cross-file references.",
            example_bad: Some("lash add \"Task\" --depends-on nonexistent.md#task:missing"),
            example_good: Some("lash add \"Task\" --depends-on tasks/setup.md#task:database-init"),
        }),

        codes::E_CREATE_WOULD_CREATE_CYCLE => Some(ErrorExplanation {
            code: codes::E_CREATE_WOULD_CREATE_CYCLE,
            summary: "Would create circular dependency",
            description: "Creating this task with the specified dependencies would create a circular dependency chain where tasks depend on each other in a loop.",
            why_it_matters: "Circular dependencies create logical impossibilities - no task in the cycle can ever be started or completed.",
            how_to_fix: "Remove the dependency that creates the cycle, or restructure the task hierarchy to break the circular reference.",
            example_bad: Some("# Task A depends on B, B depends on A:\nlash add \"Task A\" --depends-on file.md#task:b\nlash add \"Task B\" --depends-on file.md#task:a"),
            example_good: Some("# Linear dependency chain:\nlash add \"Task A\" --depends-on file.md#task:b\nlash add \"Task B\"  # No circular reference"),
        }),

        codes::E_CREATE_INVALID_POSITION => Some(ErrorExplanation {
            code: codes::E_CREATE_INVALID_POSITION,
            summary: "Insert position is invalid",
            description: "The position specified with --before or --after references a task that doesn't exist in the target file, lives at a different nesting level, or belongs to another file.",
            why_it_matters: "Task ordering requires valid position references to maintain the correct sequence.",
            how_to_fix: "Use a task ID from the target file. Both the bare ID and the qualified 'file#id' form that `lash show` prints are accepted, but the file part must name the file you are adding to. Omit these options to append at the end.",
            example_bad: Some("lash add \"Task\" --after nonexistent-task\nlash add \"Task\" -f a.md --after other-file#some-task"),
            example_good: Some("lash add \"Task\" --after existing-task-id\nlash add \"Task\" -f lash.index.md --after index#existing-task-id\nlash add \"Task\"  # Appends at end"),
        }),

        codes::E_CREATE_IO_ERROR => Some(ErrorExplanation {
            code: codes::E_CREATE_IO_ERROR,
            summary: "I/O error during task creation",
            description: "An input/output error occurred while trying to write the task to the file. This could be due to disk issues, permissions, or system problems.",
            why_it_matters: "The task could not be saved due to a system-level I/O problem.",
            how_to_fix: "Check that you have write permissions, sufficient disk space, and that the file system is accessible.",
            example_bad: None,
            example_good: None,
        }),

        _ => None,
    }
}

/// Get all available error codes that have explanations
#[must_use]
pub fn all_error_codes() -> Vec<&'static str> {
    vec![
        // Parse errors
        codes::E_PARSE_INVALID_CHECKBOX,
        codes::E_PARSE_INVALID_ANNOTATION,
        codes::E_PARSE_INVALID_HEADER,
        codes::E_PARSE_UNEXPECTED_DEPTH,
        codes::E_PARSE_INVALID_DATE,
        // Lint errors
        codes::E_LINT_DUPLICATE_ID,
        codes::E_LINT_UNKNOWN_ANNOTATION,
        codes::E_LINT_DEPTH_EXCEEDED,
        codes::E_LINT_STATUS_INCONSISTENCY,
        codes::E_LINT_INVALID_LABEL,
        codes::E_LINT_MISSING_ANNOTATION,
        codes::E_LINT_BAD_INDENTATION,
        codes::W_SEM_DOC_FRAGMENT,
        // Dependency errors
        codes::E_DEP_NOT_FOUND,
        codes::E_DEP_CYCLE,
        codes::E_DEP_INVALID_REF,
        // Index errors
        codes::E_INDEX_CORRUPTED,
        codes::E_INDEX_VERSION_MISMATCH,
        codes::E_INDEX_OUT_OF_SYNC,
        // Query errors
        codes::E_QUERY_INVALID_SYNTAX,
        codes::E_QUERY_NO_RESULTS,
        // Config errors
        codes::E_CONFIG_ROOT_NOT_FOUND,
        codes::E_CONFIG_INVALID_VALUE,
        codes::E_CONFIG_PARSE_ERROR,
        codes::E_CONFIG_MISSING_INDEX,
        // IO errors
        codes::E_IO_FILE_NOT_FOUND,
        codes::E_IO_READ_ERROR,
        codes::E_IO_WRITE_ERROR,
        codes::E_IO_PERMISSION_DENIED,
        codes::E_IO_INVALID_PATH,
        // Internal errors
        codes::E_INTERNAL,
        // Task creation errors
        codes::E_CREATE_EMPTY_TITLE,
        codes::E_CREATE_TITLE_TOO_LONG,
        codes::E_CREATE_FILE_NOT_FOUND,
        codes::E_CREATE_FILE_NOT_WRITABLE,
        codes::E_CREATE_FILE_PARSE_FAILED,
        codes::E_CREATE_PARENT_NOT_FOUND,
        codes::E_CREATE_DEPTH_LIMIT_EXCEEDED,
        codes::E_CREATE_DUPLICATE_ID,
        codes::E_CREATE_INVALID_ID_FORMAT,
        codes::E_CREATE_INVALID_LABEL,
        codes::E_CREATE_INVALID_ESTIMATE,
        codes::E_CREATE_DEPENDENCY_NOT_FOUND,
        codes::E_CREATE_WOULD_CREATE_CYCLE,
        codes::E_CREATE_INVALID_POSITION,
        codes::E_CREATE_IO_ERROR,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_codes_have_explanations() {
        for code in all_error_codes() {
            let explanation = explain_error(code);
            assert!(
                explanation.is_some(),
                "Error code {code} is listed but has no explanation"
            );
        }
    }

    #[test]
    fn test_explanation_markdown_format() {
        let explanation = explain_error(codes::E_PARSE_INVALID_CHECKBOX).unwrap();
        let markdown = explanation.to_markdown();

        assert!(markdown.contains("# Error:"));
        assert!(markdown.contains(codes::E_PARSE_INVALID_CHECKBOX));
        assert!(markdown.contains("Description:"));
        assert!(markdown.contains("Why it matters:"));
        assert!(markdown.contains("How to fix:"));
    }

    #[test]
    fn test_unknown_code_returns_none() {
        let explanation = explain_error("E_UNKNOWN_CODE");
        assert!(explanation.is_none());
    }

    #[test]
    fn test_parse_errors_have_examples() {
        let codes_with_examples = [
            codes::E_PARSE_INVALID_CHECKBOX,
            codes::E_PARSE_INVALID_ANNOTATION,
            codes::E_PARSE_INVALID_HEADER,
        ];

        for code in codes_with_examples {
            let explanation = explain_error(code).unwrap();
            assert!(
                explanation.example_bad.is_some(),
                "{code} should have bad example"
            );
            assert!(
                explanation.example_good.is_some(),
                "{code} should have good example"
            );
        }
    }
}
