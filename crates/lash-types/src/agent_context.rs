//! Agent-friendly error context enrichment
//!
//! This module enriches error diagnostics with additional context specifically
//! designed for AI agents and automated error recovery.

use crate::error::{codes, Diagnostic};
use crate::error_explanations::explain_error;

/// Enrich a diagnostic with agent-friendly context
///
/// Adds:
/// - Recovery command (exact CLI command to fix the error)
/// - Fix steps (step-by-step instructions for manual fixes)
/// - Explanation (detailed description from error explanations)
/// - Documentation URL (if available)
///
/// This function mutates the diagnostic in place and is idempotent
/// (calling it multiple times has no additional effect).
pub fn enrich_diagnostic(diagnostic: &mut Diagnostic) {
    // Skip if already enriched
    if diagnostic.recovery_command.is_some() {
        return;
    }

    // Add explanation from the error_explanations module
    if let Some(explanation) = explain_error(diagnostic.code) {
        diagnostic.explanation = Some(explanation.description.to_string());
        // Docs URL would go here once we have hosted documentation
        // diagnostic.docs_url = Some(format!("https://lash.dev/errors/{}", diagnostic.code));
    }

    // Add recovery commands and fix steps based on error code
    match diagnostic.code {
        // ===== Parse Errors =====
        codes::E_PARSE_INVALID_CHECKBOX => {
            if let Some(location) = &diagnostic.location {
                diagnostic.recovery_command = Some(format!(
                    "lash format {}",
                    shell_escape(&location.file_path.display().to_string())
                ));
            }
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Locate the invalid checkbox".to_string(),
                "Replace with valid checkbox: [ ], [x], [-], or [!]".to_string(),
                "Save the file".to_string(),
                "Alternatively, run `lash format` to fix automatically".to_string(),
            ]);
        }

        codes::E_PARSE_INVALID_ANNOTATION => {
            if let Some(location) = &diagnostic.location {
                diagnostic.recovery_command = Some(format!(
                    "lash format {}",
                    shell_escape(&location.file_path.display().to_string())
                ));
            }
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Find the malformed annotation".to_string(),
                "Ensure format is: @key: value (with space after colon)".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_PARSE_INVALID_HEADER => {
            if let Some(location) = &diagnostic.location {
                diagnostic.recovery_command = Some(format!(
                    "lash format {}",
                    shell_escape(&location.file_path.display().to_string())
                ));
            }
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Find the header without space after #".to_string(),
                "Add a space: ## Header not ##Header".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_PARSE_UNEXPECTED_DEPTH | codes::E_LINT_BAD_INDENTATION => {
            if let Some(location) = &diagnostic.location {
                diagnostic.recovery_command = Some(format!(
                    "lash format {}",
                    shell_escape(&location.file_path.display().to_string())
                ));
            }
            diagnostic.fix_steps = Some(vec![
                "Run `lash format` to automatically fix indentation".to_string(),
                "Or manually adjust to 2-space increments per nesting level".to_string(),
            ]);
        }

        codes::E_PARSE_INVALID_DATE => {
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Find the date annotation".to_string(),
                "Change to YYYY-MM-DD format (e.g., 2024-01-15)".to_string(),
                "Save the file".to_string(),
            ]);
        }

        // ===== Lint Errors =====
        codes::E_LINT_DUPLICATE_ID => {
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Find both tasks with the duplicate ID".to_string(),
                "Rename one ID to make it unique and descriptive".to_string(),
                "Update any @depends-on references to the renamed ID".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_LINT_UNKNOWN_ANNOTATION => {
            diagnostic.fix_steps = Some(vec![
                "Check for typos in the annotation name".to_string(),
                "Valid annotations: @id, @labels, @status, @owner, @estimate, @depends-on, @created, @doc, @agent-note".to_string(),
                "Either fix the typo or remove the unknown annotation".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_LINT_DEPTH_EXCEEDED => {
            diagnostic.fix_steps = Some(vec![
                "Consider flattening the task hierarchy".to_string(),
                "Move deeply nested tasks to a separate file".to_string(),
                "Use @depends-on to link to the new file".to_string(),
                "Maximum recommended depth is 4 levels".to_string(),
            ]);
        }

        codes::E_LINT_STATUS_INCONSISTENCY => {
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Option 1: Mark all child tasks as done [x] or waived [-]".to_string(),
                "Option 2: Change parent status to open [ ] or blocked [!]".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_LINT_INVALID_LABEL => {
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Find the invalid label".to_string(),
                "Use only alphanumeric characters and hyphens".to_string(),
                "Separate multiple labels with commas".to_string(),
                "Example: @labels: front-end, back-end, high-priority".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_LINT_MISSING_ANNOTATION => {
            diagnostic.fix_steps = Some(vec![
                "Open the file in your editor".to_string(),
                "Add the required annotation to the task".to_string(),
                "For @id: choose a unique, descriptive identifier".to_string(),
                "Save the file".to_string(),
            ]);
        }

        // ===== Dependency Errors =====
        codes::E_DEP_NOT_FOUND => {
            diagnostic.fix_steps = Some(vec![
                "Verify the referenced file exists".to_string(),
                "Check that the task ID exists in that file".to_string(),
                "Ensure the path is relative to project root".to_string(),
                "Fix the @depends-on annotation with correct path and ID".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_DEP_CYCLE => {
            diagnostic.fix_steps = Some(vec![
                "Review the dependency chain shown in the error".to_string(),
                "Identify which dependency creates the cycle".to_string(),
                "Remove or restructure one of the dependencies to break the cycle".to_string(),
                "Consider making tasks independent or reordering dependencies".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_DEP_INVALID_REF => {
            diagnostic.fix_steps = Some(vec![
                "Use the format: @depends-on: path/to/file.md#task:id".to_string(),
                "Path should be relative to project root".to_string(),
                "Task ID must match the @id in the target file".to_string(),
                "Save the file".to_string(),
            ]);
        }

        // ===== Index Errors =====
        codes::E_INDEX_CORRUPTED => {
            diagnostic.recovery_command = Some("lash index --rebuild".to_string());
            diagnostic.fix_steps = Some(vec![
                "Run: lash index --rebuild".to_string(),
                "This will rebuild the database from your Markdown files".to_string(),
                "All data will be reconstructed from source".to_string(),
            ]);
        }

        codes::E_INDEX_VERSION_MISMATCH => {
            diagnostic.recovery_command = Some("lash index --migrate".to_string());
            diagnostic.fix_steps = Some(vec![
                "Run: lash index --migrate".to_string(),
                "Or run: lash index --rebuild to rebuild from scratch".to_string(),
            ]);
        }

        codes::E_INDEX_OUT_OF_SYNC => {
            diagnostic.recovery_command = Some("lash index".to_string());
            diagnostic.fix_steps = Some(vec![
                "Run: lash index".to_string(),
                "This will update the index with any changed files".to_string(),
            ]);
        }

        // ===== Query Errors =====
        codes::E_QUERY_INVALID_SYNTAX => {
            diagnostic.recovery_command = Some("lash help search".to_string());
            diagnostic.fix_steps = Some(vec![
                "Check the query syntax".to_string(),
                "Run `lash help search` for query documentation".to_string(),
                "Common searches: simple text, \"exact phrase\", label:tag".to_string(),
            ]);
        }

        codes::E_QUERY_NO_RESULTS => {
            diagnostic.fix_steps = Some(vec![
                "Try broadening your search criteria".to_string(),
                "Remove some filters to see more results".to_string(),
                "Check that the tasks you're looking for exist".to_string(),
                "Run `lash list` to see all tasks".to_string(),
            ]);
        }

        // ===== Config Errors =====
        codes::E_CONFIG_ROOT_NOT_FOUND => {
            diagnostic.recovery_command = Some("lash init".to_string());
            diagnostic.fix_steps = Some(vec![
                "Run `lash init` to create a new project".to_string(),
                "Or navigate to an existing Lash project directory".to_string(),
                "Or use --root flag to specify project location".to_string(),
            ]);
        }

        codes::E_CONFIG_INVALID_VALUE => {
            diagnostic.fix_steps = Some(vec![
                "Open the configuration file in your editor".to_string(),
                "Find the invalid value mentioned in the error".to_string(),
                "Correct it according to the documentation".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_CONFIG_PARSE_ERROR => {
            diagnostic.fix_steps = Some(vec![
                "Open the configuration file in your editor".to_string(),
                "Check for TOML syntax errors".to_string(),
                "Common issues: missing quotes, incorrect indentation".to_string(),
                "Use a TOML validator if needed".to_string(),
                "Save the file".to_string(),
            ]);
        }

        codes::E_CONFIG_MISSING_INDEX => {
            diagnostic.fix_steps = Some(vec![
                "Create a file named lash.index.md or index.lash.md".to_string(),
                "Place it at the project root".to_string(),
                "Add task content following Lash format".to_string(),
            ]);
        }

        // ===== IO Errors =====
        codes::E_IO_FILE_NOT_FOUND => {
            diagnostic.fix_steps = Some(vec![
                "Check that the file path is correct".to_string(),
                "Verify the file exists at that location".to_string(),
                "If file was moved, update references to it".to_string(),
            ]);
        }

        codes::E_IO_READ_ERROR => {
            diagnostic.fix_steps = Some(vec![
                "Check file permissions".to_string(),
                "Ensure the file isn't locked by another program".to_string(),
                "Verify sufficient disk space".to_string(),
            ]);
        }

        codes::E_IO_WRITE_ERROR => {
            diagnostic.fix_steps = Some(vec![
                "Check that you have write permissions".to_string(),
                "Ensure sufficient disk space".to_string(),
                "Verify the file isn't read-only".to_string(),
            ]);
        }

        codes::E_IO_PERMISSION_DENIED => {
            diagnostic.fix_steps = Some(vec![
                "Check file and directory permissions".to_string(),
                "You may need to run with different permissions".to_string(),
                "Or adjust file ownership/permissions".to_string(),
            ]);
        }

        codes::E_IO_INVALID_PATH => {
            diagnostic.fix_steps = Some(vec![
                "Check that the path is properly formatted".to_string(),
                "Ensure no invalid characters for your OS".to_string(),
                "Verify the path structure is correct".to_string(),
            ]);
        }

        // ===== Internal Errors =====
        codes::E_INTERNAL => {
            diagnostic.fix_steps = Some(vec![
                "This is an internal error - likely a bug in Lash".to_string(),
                "Please report this issue on the Lash issue tracker".to_string(),
                "Include the full error message and steps to reproduce".to_string(),
            ]);
        }

        _ => {
            // Unknown error code - no specific recovery guidance
        }
    }
}

/// Escape a string for safe use in shell commands
fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('\'') || s.contains('"') || s.contains('\\') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{LashError, Location};
    use std::path::PathBuf;

    #[test]
    fn test_enrich_parse_error() {
        let err = LashError::parse_invalid_checkbox(PathBuf::from("test.md"), 10, 5, "[*] invalid");
        let mut diag = err.to_diagnostic();

        enrich_diagnostic(&mut diag);

        assert!(diag.recovery_command.is_some());
        assert!(diag.fix_steps.is_some());
        assert!(diag.explanation.is_some());
        assert!(diag
            .recovery_command
            .as_ref()
            .unwrap()
            .contains("lash format"));
    }

    #[test]
    fn test_enrich_index_error() {
        let err = LashError::index_corrupted("details");
        let mut diag = err.to_diagnostic();

        enrich_diagnostic(&mut diag);

        assert!(diag.recovery_command.is_some());
        assert_eq!(
            diag.recovery_command.as_ref().unwrap(),
            "lash index --rebuild"
        );
        assert!(diag.fix_steps.is_some());
    }

    #[test]
    fn test_enrich_is_idempotent() {
        let err = LashError::index_out_of_sync(5);
        let mut diag = err.to_diagnostic();

        enrich_diagnostic(&mut diag);
        let recovery1 = diag.recovery_command.clone();

        enrich_diagnostic(&mut diag);
        let recovery2 = diag.recovery_command.clone();

        assert_eq!(recovery1, recovery2);
    }

    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("simple"), "simple");
        assert_eq!(shell_escape("with space"), "'with space'");
        assert_eq!(shell_escape("with'quote"), "'with'\\''quote'");
    }

    #[test]
    fn test_all_error_codes_get_enriched() {
        use crate::error_explanations::all_error_codes;

        for code in all_error_codes() {
            // Create a minimal diagnostic for each code
            let mut diag = Diagnostic {
                code,
                severity: crate::error::Severity::Error,
                message: "test".to_string(),
                location: Some(Location::new(PathBuf::from("test.md"), 1, 1)),
                snippet: None,
                help: None,
                labels: None,
                recovery_command: None,
                fix_steps: None,
                explanation: None,
                docs_url: None,
            };

            enrich_diagnostic(&mut diag);

            // All errors should get an explanation at minimum
            assert!(
                diag.explanation.is_some(),
                "Error code {code} should have an explanation"
            );
        }
    }
}
