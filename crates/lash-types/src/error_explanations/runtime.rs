//! Explanations for runtime error codes
//!
//! Covers dependency resolution (`E_DEP_*`), the `SQLite` index (`E_INDEX_*`),
//! queries (`E_QUERY_*`), configuration (`E_CONFIG_*`), I/O (`E_IO_*`) and
//! internal failures.

use super::ErrorExplanation;
use crate::error::codes;

/// Codes explained by this module
pub(super) const CODES: &[&str] = &[
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
];

/// Look up a runtime error explanation
pub(super) fn explain(code: &str) -> Option<ErrorExplanation> {
    match code {
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
        _ => None,
    }
}
