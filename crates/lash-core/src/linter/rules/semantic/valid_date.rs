//! Date format validation rule
//!
//! Ensures that date annotations (like @created) match the YYYY-MM-DD format
//! and represent valid dates.

use lash_types::{Severity, TaskFile};

use crate::linter::{Fix, LintContext, LintDiagnostic, LintRule, Replacement};

/// Rule that validates date format
///
/// Dates must follow the ISO 8601 format: YYYY-MM-DD
/// - Year must be 4 digits
/// - Month must be 01-12
/// - Day must be valid for the given month
///
/// **Code:** `E_SEM_INVALID_DATE`
/// **Severity:** Error
///
/// # Auto-fix
///
/// The auto-fix attempts to parse common date formats and convert them
/// to YYYY-MM-DD. If the date cannot be parsed, no fix is provided.
///
/// # Examples
///
/// Valid dates:
/// ```markdown
/// @created: 2024-01-15
/// @created: 2024-12-31
/// ```
///
/// Invalid dates:
/// ```markdown
/// @created: 2024-1-15 ← month should be zero-padded
/// @created: 01/15/2024 ← wrong format
/// @created: 2024-02-30 ← invalid day
/// @created: 2024-13-01 ← invalid month
/// ```
pub struct ValidDateRule;

impl ValidDateRule {
    /// Create a new valid date rule
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a date string is in valid YYYY-MM-DD format
    fn is_valid_date(date: &str) -> bool {
        // Check format: YYYY-MM-DD
        if date.len() != 10 {
            return false;
        }

        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() != 3 {
            return false;
        }

        // Parse year, month, day
        let year = parts[0].parse::<u32>().ok();
        let month = parts[1].parse::<u32>().ok();
        let day = parts[2].parse::<u32>().ok();

        if year.is_none() || month.is_none() || day.is_none() {
            return false;
        }

        let year = year.unwrap();
        let month = month.unwrap();
        let day = day.unwrap();

        // Validate ranges
        if !(1000..=9999).contains(&year) {
            return false;
        }

        if !(1..=12).contains(&month) {
            return false;
        }

        // Check day is valid for month
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                // Leap year calculation
                if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            _ => return false,
        };

        day >= 1 && day <= days_in_month
    }

    /// Attempt to parse and reformat a date string
    fn try_reformat_date(date: &str) -> Option<String> {
        // Try to parse common formats and convert to YYYY-MM-DD

        // Already in correct format?
        if Self::is_valid_date(date) {
            return Some(date.to_string());
        }

        // Try MM/DD/YYYY format
        if let Some(reformatted) = Self::parse_mdy_format(date) {
            if Self::is_valid_date(&reformatted) {
                return Some(reformatted);
            }
        }

        // Try DD/MM/YYYY format (ambiguous, but common in some regions)
        if let Some(reformatted) = Self::parse_dmy_format(date) {
            if Self::is_valid_date(&reformatted) {
                return Some(reformatted);
            }
        }

        // Try YYYY-M-D or YYYY-MM-D or YYYY-M-DD (missing zero padding)
        if let Some(reformatted) = Self::parse_unpadded_ymd(date) {
            if Self::is_valid_date(&reformatted) {
                return Some(reformatted);
            }
        }

        None
    }

    fn parse_mdy_format(date: &str) -> Option<String> {
        let parts: Vec<&str> = date.split('/').collect();
        if parts.len() == 3 {
            if let (Ok(month), Ok(day), Ok(year)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                return Some(format!("{year:04}-{month:02}-{day:02}"));
            }
        }
        None
    }

    fn parse_dmy_format(date: &str) -> Option<String> {
        let parts: Vec<&str> = date.split('/').collect();
        if parts.len() == 3 {
            if let (Ok(day), Ok(month), Ok(year)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                return Some(format!("{year:04}-{month:02}-{day:02}"));
            }
        }
        None
    }

    fn parse_unpadded_ymd(date: &str) -> Option<String> {
        let parts: Vec<&str> = date.split('-').collect();
        if parts.len() == 3 {
            if let (Ok(year), Ok(month), Ok(day)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                return Some(format!("{year:04}-{month:02}-{day:02}"));
            }
        }
        None
    }
}

impl Default for ValidDateRule {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRule for ValidDateRule {
    fn code(&self) -> &'static str {
        "E_SEM_INVALID_DATE"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn name(&self) -> String {
        "Date format".to_string()
    }

    fn description(&self) -> &'static str {
        "Ensures dates match YYYY-MM-DD format and are valid"
    }

    fn check_file(&self, file: &TaskFile, ctx: &LintContext) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check file-level created date
        if let Some(created) = &file.metadata.created {
            if !Self::is_valid_date(created) {
                let mut diag = LintDiagnostic::error(
                    self.code(),
                    format!("Invalid date format: '{created}'"),
                    ctx.file_path.clone(),
                    0,
                    0,
                )
                .with_help("Use YYYY-MM-DD format (e.g., 2024-01-15)");

                // Try to provide auto-fix
                if let Some(reformatted) = Self::try_reformat_date(created) {
                    diag = diag.with_fix(Fix {
                        description: format!("Reformat '{created}' to '{reformatted}'"),
                        replacement: Replacement::TextReplace {
                            old: format!("@created: {created}"),
                            new: format!("@created: {reformatted}"),
                        },
                    });
                }

                diagnostics.push(diag);
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lash_types::{FileMetadata, LashConfig, TaskTree};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_config() -> LashConfig {
        LashConfig {
            root_path: PathBuf::from("/test"),
            index_file: "index.md".to_string(),
            max_depth: 2,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/test.db"),
            custom_annotation_keys: vec![],
        }
    }

    fn make_file_with_date(date: Option<&str>) -> TaskFile {
        TaskFile {
            path: PathBuf::from("test.md"),
            title: "Test File".to_string(),
            id: "test".to_string(),
            metadata: FileMetadata {
                created: date.map(std::string::ToString::to_string),
                ..Default::default()
            },
            description: None,
            description_agent_notes: Vec::new(),
            tasks: TaskTree::new(),
            hash: "hash".to_string(),
            mtime: SystemTime::now(),
        }
    }

    #[test]
    fn test_valid_dates() {
        let rule = ValidDateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let valid_dates = vec![
            "2024-01-15",
            "2024-12-31",
            "2000-02-29", // Leap year
            "1999-01-01",
            "2024-06-15",
        ];

        for date in valid_dates {
            let file = make_file_with_date(Some(date));
            let diagnostics = rule.check_file(&file, &ctx);
            assert!(diagnostics.is_empty(), "Date '{date}' should be valid");
        }
    }

    #[test]
    fn test_invalid_date_format() {
        let rule = ValidDateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_date(Some("2024-1-15"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "E_SEM_INVALID_DATE");
        assert!(diagnostics[0].message.contains("2024-1-15"));
        assert!(diagnostics[0].has_fix());
    }

    #[test]
    fn test_invalid_month() {
        let rule = ValidDateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_date(Some("2024-13-01"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_invalid_day() {
        let rule = ValidDateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // February 30th doesn't exist
        let file = make_file_with_date(Some("2024-02-30"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);

        // April 31st doesn't exist
        let file = make_file_with_date(Some("2024-04-31"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_leap_year() {
        let rule = ValidDateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        // 2024 is a leap year (Feb 29 valid)
        let file = make_file_with_date(Some("2024-02-29"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());

        // 2023 is not a leap year (Feb 29 invalid)
        let file = make_file_with_date(Some("2023-02-29"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);

        // 2000 is a leap year (divisible by 400)
        let file = make_file_with_date(Some("2000-02-29"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty());

        // 1900 is not a leap year (divisible by 100 but not 400)
        let file = make_file_with_date(Some("1900-02-29"));
        let diagnostics = rule.check_file(&file, &ctx);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_no_date() {
        let rule = ValidDateRule::new();
        let config = make_config();
        let files = HashMap::new();
        let ctx = LintContext::new(&config, PathBuf::from("test.md"), &files);

        let file = make_file_with_date(None);
        let diagnostics = rule.check_file(&file, &ctx);
        assert!(diagnostics.is_empty(), "No date means no error");
    }

    #[test]
    fn test_is_valid_date() {
        assert!(ValidDateRule::is_valid_date("2024-01-15"));
        assert!(ValidDateRule::is_valid_date("2024-12-31"));
        assert!(ValidDateRule::is_valid_date("2000-02-29"));

        assert!(!ValidDateRule::is_valid_date("2024-1-15"));
        assert!(!ValidDateRule::is_valid_date("2024-13-01"));
        assert!(!ValidDateRule::is_valid_date("2024-02-30"));
        assert!(!ValidDateRule::is_valid_date("01/15/2024"));
        assert!(!ValidDateRule::is_valid_date("invalid"));
    }

    #[test]
    fn test_try_reformat_date() {
        // Unpadded format
        assert_eq!(
            ValidDateRule::try_reformat_date("2024-1-15"),
            Some("2024-01-15".to_string())
        );

        // MM/DD/YYYY format
        assert_eq!(
            ValidDateRule::try_reformat_date("01/15/2024"),
            Some("2024-01-15".to_string())
        );

        // Already valid
        assert_eq!(
            ValidDateRule::try_reformat_date("2024-01-15"),
            Some("2024-01-15".to_string())
        );

        // Invalid date
        assert_eq!(ValidDateRule::try_reformat_date("invalid"), None);
        assert_eq!(ValidDateRule::try_reformat_date("2024-13-01"), None);
    }

    #[test]
    fn test_rule_metadata() {
        let rule = ValidDateRule::new();
        assert_eq!(rule.code(), "E_SEM_INVALID_DATE");
        assert_eq!(rule.severity(), Severity::Error);
        assert_eq!(rule.name(), "Date format");
        assert!(!rule.description().is_empty());
    }
}
