//! Error validation after applying fixes
//!
//! This module provides the `ErrorValidator` which re-lints files after fixes
//! are applied to verify that errors were actually fixed and to detect any
//! new errors introduced by the fix process.
//!
//! # Example
//!
//! ```no_run
//! use lash::error_validator::ErrorValidator;
//! use lash_core::linter::LintDiagnostic;
//! use std::path::Path;
//!
//! let validator = ErrorValidator::new();
//! let path = Path::new("tasks.md");
//! let original_diagnostics: Vec<LintDiagnostic> = vec![];
//!
//! // After applying fixes to the file...
//! let result = validator.validate_file(path, &original_diagnostics).unwrap();
//!
//! println!("Fixed: {}, Remaining: {}, New: {}",
//!     result.fixed_errors.len(),
//!     result.remaining_errors.len(),
//!     result.new_errors.len()
//! );
//! ```

use lash_core::linter::LintDiagnostic;
use lash_core::parser::parse_file_from_string;
use lash_types::{LashConfig, LashError};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Result of validating a file after fixes were applied
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    /// Errors that still exist after fixes (same code+location as original)
    pub remaining_errors: Vec<LintDiagnostic>,

    /// Descriptions of errors that were successfully fixed
    pub fixed_errors: Vec<String>,

    /// New errors that appeared after fixing (not in original set)
    pub new_errors: Vec<LintDiagnostic>,

    /// Total number of errors before fixes
    pub total_before: usize,

    /// Total number of errors after fixes
    pub total_after: usize,
}

impl ValidationResult {
    /// Check if all errors were fixed (no remaining, no new errors)
    #[must_use]
    pub fn is_fully_fixed(&self) -> bool {
        self.remaining_errors.is_empty() && self.new_errors.is_empty()
    }

    /// Check if validation succeeded (fewer or equal errors, no new errors)
    #[must_use]
    pub fn is_improved(&self) -> bool {
        self.new_errors.is_empty() && self.total_after <= self.total_before
    }

    /// Get the number of errors that were fixed
    #[must_use]
    pub fn fixed_count(&self) -> usize {
        self.fixed_errors.len()
    }
}

/// Validates files after fixes are applied
///
/// The validator re-parses and re-lints files, then compares the new
/// errors with the original set to determine:
/// - Which errors were fixed
/// - Which errors remain
/// - Which new errors were introduced
pub struct ErrorValidator {
    /// Configuration for parsing and linting
    config: LashConfig,
}

impl ErrorValidator {
    /// Create a new error validator with default configuration
    ///
    /// # Example
    ///
    /// ```
    /// use lash::error_validator::ErrorValidator;
    ///
    /// let validator = ErrorValidator::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: LashConfig::default(),
        }
    }

    /// Create a new error validator with custom configuration
    ///
    /// # Example
    ///
    /// ```
    /// use lash::error_validator::ErrorValidator;
    /// use lash_types::LashConfig;
    ///
    /// let config = LashConfig::default();
    /// let validator = ErrorValidator::with_config(config);
    /// ```
    #[must_use]
    pub fn with_config(config: LashConfig) -> Self {
        Self { config }
    }

    /// Re-lint content after fixes and compare with original errors
    ///
    /// This method parses the fixed content and compares the new set of
    /// errors with the original set to determine what was fixed, what
    /// remains, and what new errors appeared.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file (used for error messages)
    /// * `fixed_content` - The content after fixes were applied
    /// * `original_diagnostics` - The original errors before fixes
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` containing the comparison results,
    /// or an error if parsing the fixed content failed.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Fixed content cannot be parsed (fix corrupted the file)
    /// - Internal validation errors occur
    ///
    /// # Example
    ///
    /// ```
    /// use lash::error_validator::ErrorValidator;
    /// use lash_core::linter::LintDiagnostic;
    /// use std::path::Path;
    ///
    /// let validator = ErrorValidator::new();
    /// let fixed_content = "# Test\n\n## Tasks\n\n- [ ] task";
    /// let original_errors: Vec<LintDiagnostic> = vec![];
    ///
    /// let result = validator.validate_content(
    ///     Path::new("test.md"),
    ///     fixed_content,
    ///     &original_errors
    /// ).unwrap();
    ///
    /// assert!(result.is_fully_fixed());
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn validate_content(
        &self,
        file_path: &Path,
        fixed_content: &str,
        original_diagnostics: &[LintDiagnostic],
    ) -> lash_types::Result<ValidationResult> {
        // Parse the fixed content
        // If parsing fails, we get diagnostics from the parser
        let new_diagnostics = match parse_file_from_string(fixed_content, &self.config) {
            Ok(_) => {
                // Parse succeeded, no errors
                Vec::new()
            }
            Err(parse_diagnostics) => {
                // Parse failed, convert parser diagnostics to lint diagnostics
                parse_diagnostics
                    .into_iter()
                    .map(|d| Self::diagnostic_to_lint_diagnostic(file_path, d))
                    .collect()
            }
        };

        // Compare original and new diagnostics
        Ok(Self::compare_diagnostics(
            original_diagnostics,
            &new_diagnostics,
        ))
    }

    /// High-level validation after applying fixes to a file
    ///
    /// This method reads the file from disk, parses it, and compares
    /// the new errors with the original set.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to validate
    /// * `original_diagnostics` - The original errors before fixes
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` or an error if the file cannot be read.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File cannot be read from disk
    /// - File cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash::error_validator::ErrorValidator;
    /// use lash_core::linter::LintDiagnostic;
    /// use std::path::Path;
    ///
    /// let validator = ErrorValidator::new();
    /// let original_errors: Vec<LintDiagnostic> = vec![];
    ///
    /// let result = validator.validate_file(
    ///     Path::new("tasks.md"),
    ///     &original_errors
    /// ).unwrap();
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn validate_file(
        &self,
        file_path: &Path,
        original_diagnostics: &[LintDiagnostic],
    ) -> lash_types::Result<ValidationResult> {
        // Read the file
        let content = fs::read_to_string(file_path).map_err(|e| LashError::IO {
            code: "E_IO_READ_FAILED",
            message: format!(
                "Failed to read file for validation: {}",
                file_path.display()
            ),
            path: Some(file_path.to_path_buf()),
            io_error: Some(e.to_string()),
        })?;

        // Validate the content
        self.validate_content(file_path, &content, original_diagnostics)
    }

    /// Compare two sets of diagnostics to determine what was fixed/added
    fn compare_diagnostics(
        original: &[LintDiagnostic],
        new: &[LintDiagnostic],
    ) -> ValidationResult {
        let total_before = original.len();
        let total_after = new.len();

        // Create sets of error signatures for comparison
        // We compare by code + location (file, line, column)
        let original_sigs: HashSet<ErrorSignature> = original
            .iter()
            .map(ErrorSignature::from_diagnostic)
            .collect();

        let new_sigs: HashSet<ErrorSignature> =
            new.iter().map(ErrorSignature::from_diagnostic).collect();

        // Find remaining errors (in both original and new)
        let remaining_errors: Vec<LintDiagnostic> = new
            .iter()
            .filter(|d| {
                let sig = ErrorSignature::from_diagnostic(d);
                original_sigs.contains(&sig)
            })
            .cloned()
            .collect();

        // Find fixed errors (in original but not in new)
        let fixed_errors: Vec<String> = original
            .iter()
            .filter(|d| {
                let sig = ErrorSignature::from_diagnostic(d);
                !new_sigs.contains(&sig)
            })
            .map(|d| format!("{} at line {}", d.code, d.location.line.unwrap_or(0)))
            .collect();

        // Find new errors (in new but not in original)
        let new_errors: Vec<LintDiagnostic> = new
            .iter()
            .filter(|d| {
                let sig = ErrorSignature::from_diagnostic(d);
                !original_sigs.contains(&sig)
            })
            .cloned()
            .collect();

        ValidationResult {
            remaining_errors,
            fixed_errors,
            new_errors,
            total_before,
            total_after,
        }
    }

    /// Convert a parser `Diagnostic` to a `LintDiagnostic`
    fn diagnostic_to_lint_diagnostic(
        file_path: &Path,
        diag: lash_types::Diagnostic,
    ) -> LintDiagnostic {
        use lash_types::Severity;

        let location = diag
            .location
            .unwrap_or_else(|| lash_types::Location::new(file_path.to_path_buf(), 0, 0));

        let mut lint_diag = match diag.severity {
            Severity::Error => LintDiagnostic::error(
                diag.code,
                diag.message,
                location.file_path,
                location.line.unwrap_or(0),
                location.column.unwrap_or(0),
            ),
            Severity::Warning => LintDiagnostic::warning(
                diag.code,
                diag.message,
                location.file_path,
                location.line.unwrap_or(0),
                location.column.unwrap_or(0),
            ),
            Severity::Info | Severity::Hint => LintDiagnostic::info(
                diag.code,
                diag.message,
                location.file_path,
                location.line.unwrap_or(0),
                location.column.unwrap_or(0),
            ),
        };

        // Add optional fields
        if let Some(snippet) = diag.snippet {
            lint_diag = lint_diag.with_snippet(snippet);
        }
        if let Some(help) = diag.help {
            lint_diag = lint_diag.with_help(help);
        }

        lint_diag
    }
}

impl Default for ErrorValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// A signature identifying a unique error for comparison
///
/// We use code + location to determine if two errors are "the same"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ErrorSignature {
    code: String,
    file_path: String,
    line: usize,
    column: usize,
}

impl ErrorSignature {
    /// Create an error signature from a diagnostic
    fn from_diagnostic(diag: &LintDiagnostic) -> Self {
        Self {
            code: diag.code.to_string(),
            file_path: diag.location.file_path.display().to_string(),
            line: diag.location.line.unwrap_or(0),
            column: diag.location.column.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_core::linter::Fix;
    use std::path::PathBuf;

    fn make_diagnostic(code: &'static str, line: usize, column: usize) -> LintDiagnostic {
        LintDiagnostic::error(
            code,
            format!("Test error {code}"),
            PathBuf::from("test.md"),
            line,
            column,
        )
    }

    #[test]
    fn test_validation_result_is_fully_fixed() {
        let result = ValidationResult {
            remaining_errors: vec![],
            fixed_errors: vec!["E001".to_string()],
            new_errors: vec![],
            total_before: 1,
            total_after: 0,
        };

        assert!(result.is_fully_fixed());
        assert!(result.is_improved());
        assert_eq!(result.fixed_count(), 1);
    }

    #[test]
    fn test_validation_result_has_remaining() {
        let result = ValidationResult {
            remaining_errors: vec![make_diagnostic("E001", 1, 1)],
            fixed_errors: vec![],
            new_errors: vec![],
            total_before: 1,
            total_after: 1,
        };

        assert!(!result.is_fully_fixed());
        assert!(result.is_improved());
        assert_eq!(result.fixed_count(), 0);
    }

    #[test]
    fn test_validation_result_has_new_errors() {
        let result = ValidationResult {
            remaining_errors: vec![],
            fixed_errors: vec!["E001".to_string()],
            new_errors: vec![make_diagnostic("E002", 2, 1)],
            total_before: 1,
            total_after: 1,
        };

        assert!(!result.is_fully_fixed());
        assert!(!result.is_improved()); // New errors = not improved
    }

    #[test]
    fn test_error_signature_equality() {
        let diag1 = make_diagnostic("E001", 5, 10);
        let diag2 = make_diagnostic("E001", 5, 10);
        let diag3 = make_diagnostic("E001", 5, 11); // Different column
        let diag4 = make_diagnostic("E002", 5, 10); // Different code

        let sig1 = ErrorSignature::from_diagnostic(&diag1);
        let sig2 = ErrorSignature::from_diagnostic(&diag2);
        let sig3 = ErrorSignature::from_diagnostic(&diag3);
        let sig4 = ErrorSignature::from_diagnostic(&diag4);

        assert_eq!(sig1, sig2);
        assert_ne!(sig1, sig3);
        assert_ne!(sig1, sig4);
    }

    #[test]
    fn test_compare_diagnostics_all_fixed() {
        let original = vec![make_diagnostic("E001", 1, 1), make_diagnostic("E002", 2, 1)];

        let new = vec![];

        let result = ErrorValidator::compare_diagnostics(&original, &new);

        assert_eq!(result.total_before, 2);
        assert_eq!(result.total_after, 0);
        assert_eq!(result.fixed_errors.len(), 2);
        assert_eq!(result.remaining_errors.len(), 0);
        assert_eq!(result.new_errors.len(), 0);
        assert!(result.is_fully_fixed());
    }

    #[test]
    fn test_compare_diagnostics_some_remaining() {
        let original = vec![make_diagnostic("E001", 1, 1), make_diagnostic("E002", 2, 1)];

        let new = vec![make_diagnostic("E002", 2, 1)];

        let result = ErrorValidator::compare_diagnostics(&original, &new);

        assert_eq!(result.total_before, 2);
        assert_eq!(result.total_after, 1);
        assert_eq!(result.fixed_errors.len(), 1);
        assert_eq!(result.remaining_errors.len(), 1);
        assert_eq!(result.new_errors.len(), 0);
        assert!(!result.is_fully_fixed());
        assert!(result.is_improved());
    }

    #[test]
    fn test_compare_diagnostics_new_errors_introduced() {
        let original = vec![make_diagnostic("E001", 1, 1)];

        let new = vec![
            make_diagnostic("E001", 1, 1), // Remaining
            make_diagnostic("E002", 2, 1), // New
        ];

        let result = ErrorValidator::compare_diagnostics(&original, &new);

        assert_eq!(result.total_before, 1);
        assert_eq!(result.total_after, 2);
        assert_eq!(result.fixed_errors.len(), 0);
        assert_eq!(result.remaining_errors.len(), 1);
        assert_eq!(result.new_errors.len(), 1);
        assert!(!result.is_fully_fixed());
        assert!(!result.is_improved()); // More errors = not improved
    }

    #[test]
    fn test_compare_diagnostics_different_locations() {
        let original = vec![make_diagnostic("E001", 1, 1)];

        // Same code, different location = different error
        let new = vec![make_diagnostic("E001", 2, 1)];

        let result = ErrorValidator::compare_diagnostics(&original, &new);

        assert_eq!(result.fixed_errors.len(), 1); // Original at line 1 fixed
        assert_eq!(result.new_errors.len(), 1); // New at line 2
        assert_eq!(result.remaining_errors.len(), 0);
    }

    #[test]
    fn test_validate_content_valid() {
        let validator = ErrorValidator::new();

        let content = r"# Test File

## Tasks

- [ ] Task 1
- [x] Task 2
";

        let original_diags = vec![make_diagnostic("E_TEST", 5, 3)];

        let result = validator
            .validate_content(Path::new("test.md"), content, &original_diags)
            .unwrap();

        // Valid content should parse without errors
        assert_eq!(result.total_after, 0);
        assert_eq!(result.fixed_errors.len(), 1);
    }

    #[test]
    fn test_validate_content_parse_error() {
        let validator = ErrorValidator::new();

        // Content with invalid checkbox
        let content = r"# Test File

## Tasks

- [*] Invalid checkbox
";

        let original_diags = vec![];

        let result = validator
            .validate_content(Path::new("test.md"), content, &original_diags)
            .unwrap();

        // Should detect parse error
        assert!(result.total_after > 0);
        assert_eq!(result.new_errors.len(), result.total_after);
    }

    #[test]
    fn test_validate_content_fix_introduces_error() {
        let validator = ErrorValidator::new();

        // Original had one error, fix introduced a different error
        let content = r"# Test File

## Tasks

- [?] Different invalid checkbox
";

        let original_diags = vec![make_diagnostic("E001", 5, 3)];

        let result = validator
            .validate_content(Path::new("test.md"), content, &original_diags)
            .unwrap();

        // Original error should be "fixed" (not present)
        // But new parse error introduced
        assert_eq!(result.fixed_errors.len(), 1);
        assert!(!result.new_errors.is_empty());
        assert!(!result.is_improved()); // Has new errors
    }

    #[test]
    fn test_validate_file_not_found() {
        let validator = ErrorValidator::new();
        let result = validator.validate_file(Path::new("/nonexistent.md"), &[]);

        assert!(result.is_err());
        match result {
            Err(LashError::IO { code, .. }) => {
                assert_eq!(code, "E_IO_READ_FAILED");
            }
            _ => panic!("Expected IO error"),
        }
    }

    #[test]
    fn test_validator_with_custom_config() {
        let config = LashConfig {
            max_depth: 10,
            ..Default::default()
        };

        let validator = ErrorValidator::with_config(config);
        assert_eq!(validator.config.max_depth, 10);
    }

    #[test]
    fn test_validator_default() {
        let validator = ErrorValidator::default();
        assert_eq!(validator.config.max_depth, LashConfig::default().max_depth);
    }

    #[test]
    fn test_diagnostic_conversion() {
        use lash_types::{Diagnostic, Location, Severity};

        let parser_diag = Diagnostic {
            severity: Severity::Error,
            code: "E_TEST",
            message: "Test message".to_string(),
            location: Some(Location::new(PathBuf::from("test.md"), 5, 10)),
            snippet: Some("snippet".to_string()),
            help: Some("help text".to_string()),
            labels: None,
            recovery_command: None,
            fix_steps: None,
            explanation: None,
            docs_url: None,
        };

        let lint_diag =
            ErrorValidator::diagnostic_to_lint_diagnostic(Path::new("test.md"), parser_diag);

        assert_eq!(lint_diag.code, "E_TEST");
        assert_eq!(lint_diag.message, "Test message");
        assert_eq!(lint_diag.location.line, Some(5));
        assert_eq!(lint_diag.location.column, Some(10));
        assert_eq!(lint_diag.snippet, Some("snippet".to_string()));
        assert_eq!(lint_diag.help, Some("help text".to_string()));
    }

    #[test]
    fn test_compare_diagnostics_with_fixes_attached() {
        // Create diagnostics with fixes attached
        let fix1 = Fix::replace("fix error", "old", "new");
        let diag1 = make_diagnostic("E001", 1, 1).with_fix(fix1);

        let original = vec![diag1];
        let new = vec![];

        let result = ErrorValidator::compare_diagnostics(&original, &new);

        // Should still properly detect that error was fixed
        assert_eq!(result.fixed_errors.len(), 1);
        assert_eq!(result.remaining_errors.len(), 0);
    }

    #[test]
    fn test_multiple_errors_same_line_different_columns() {
        let original = vec![
            make_diagnostic("E001", 5, 1),
            make_diagnostic("E002", 5, 10),
        ];

        let new = vec![make_diagnostic("E002", 5, 10)];

        let result = ErrorValidator::compare_diagnostics(&original, &new);

        // E001 at column 1 should be fixed
        // E002 at column 10 should remain
        assert_eq!(result.fixed_errors.len(), 1);
        assert_eq!(result.remaining_errors.len(), 1);
    }
}
