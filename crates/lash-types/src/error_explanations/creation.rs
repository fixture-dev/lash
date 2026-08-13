//! Explanations for task creation error codes (`E_CREATE_*`)

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
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
];

/// Look up a task creation error explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
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
