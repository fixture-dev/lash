//! Detailed error explanations for the `lash explain` command
//!
//! This module provides comprehensive documentation for each error code,
//! including:
//! - What the error means
//! - Why it occurs
//! - How to fix it
//! - Examples of the error and correct code
//!
//! Explanations are grouped into submodules by the surface that emits them:
//! parser codes, linter rule codes (syntax, semantic, cross-file), runtime
//! errors, and task-creation errors. Every code the linter can emit must have
//! an explanation here — `lash lint` tells users to run `lash explain <CODE>`,
//! and that advice is only worth following if the code is known.

mod creation;
mod crossfile;
mod legacy_lint;
mod parse;
mod runtime;
mod semantic;
mod syntax;

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
    parse::explain(code)
        .or_else(|| legacy_lint::explain(code))
        .or_else(|| syntax::explain(code))
        .or_else(|| semantic::explain(code))
        .or_else(|| crossfile::explain(code))
        .or_else(|| runtime::explain(code))
        .or_else(|| creation::explain(code))
}

/// Get all available error codes that have explanations
#[must_use]
pub fn all_error_codes() -> Vec<&'static str> {
    let mut codes = Vec::new();
    for group in [
        parse::CODES,
        legacy_lint::CODES,
        syntax::CODES,
        semantic::CODES,
        crossfile::CODES,
        runtime::CODES,
        creation::CODES,
    ] {
        codes.extend_from_slice(group);
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::codes;

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
    fn test_explanation_code_matches_lookup_key() {
        for code in all_error_codes() {
            let explanation = explain_error(code).unwrap();
            assert_eq!(
                explanation.code, code,
                "explain_error({code}) returned an explanation for {}",
                explanation.code
            );
        }
    }

    #[test]
    fn test_no_duplicate_codes() {
        let codes = all_error_codes();
        let mut seen = std::collections::HashSet::new();
        for code in &codes {
            assert!(seen.insert(*code), "Error code {code} is listed twice");
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

    // GitHub issue #58: `lash lint` emits W_INDEX_ORPHAN and E_LINK_NOT_FOUND,
    // and the diagnostic footer points at `lash explain`. Before this, explain
    // knew none of the linter's own codes.
    #[test]
    fn test_linter_rule_codes_are_explained() {
        for code in [
            codes::W_INDEX_ORPHAN,
            codes::E_LINK_NOT_FOUND,
            codes::E_SYNTAX_CHECKBOX,
            codes::E_SEM_DUPLICATE_ID,
            codes::W_NOTE_TOO_LONG,
            codes::I_SYNTAX_ORDER,
        ] {
            assert!(
                explain_error(code).is_some(),
                "linter code {code} must be explainable"
            );
        }
    }

    // The orphan explanation is the one place a user who hit the warning can
    // learn that `.lashignore` exists.
    #[test]
    fn test_orphan_explanation_mentions_lashignore() {
        let explanation = explain_error(codes::W_INDEX_ORPHAN).unwrap();
        assert!(
            explanation.how_to_fix.contains(".lashignore"),
            "W_INDEX_ORPHAN must point at .lashignore"
        );
    }
}
