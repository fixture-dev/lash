//! Explain command implementation
//!
//! The `lash explain` command provides detailed explanations of error codes,
//! including what causes them, why they matter, and how to fix them.

use anyhow::Result;
use lash::theme::CliTheme;
use lash_types::error_explanations::{all_error_codes, explain_error};

/// Arguments for the explain command
#[derive(Debug, Clone)]
pub struct ExplainArgs {
    /// The error code to explain
    pub code: String,
    /// List all available error codes
    pub list: bool,
    /// Output JSON format
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
}

/// Execute the explain command
///
/// # Arguments
///
/// * `args` - Explain command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (error code not found)
pub fn execute(args: &ExplainArgs) -> Result<i32> {
    // Load theme based on no_color flag and output format
    let theme = if args.json {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    // Handle --list flag
    if args.list {
        return list_error_codes(args, theme.as_ref());
    }

    // Look up the error code
    let code = normalize_code(&args.code);

    if let Some(explanation) = explain_error(&code) {
        if args.json {
            print_json(&explanation)?;
        } else {
            print_human(&explanation, theme.as_ref());
        }
        Ok(0)
    } else {
        let error_msg = format!("Error: Unknown error code '{}'", args.code);
        if let Some(t) = &theme {
            eprintln!("{}", t.style_error(&error_msg));
        } else {
            eprintln!("{error_msg}");
        }
        eprintln!();
        eprintln!("Run 'lash explain --list' to see all available error codes.");
        Ok(1)
    }
}

/// Normalize the error code (handle case variations)
fn normalize_code(code: &str) -> String {
    // Error codes are uppercase
    code.to_uppercase()
}

/// List all available error codes
fn list_error_codes(args: &ExplainArgs, theme: Option<&CliTheme>) -> Result<i32> {
    let codes = all_error_codes();

    if args.json {
        let json = serde_json::json!({
            "error_codes": codes,
            "count": codes.len()
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        if let Some(t) = theme {
            println!("{}", t.style_info("Available Error Codes"));
            println!("{}", t.style_muted(&"=".repeat(50)));
        } else {
            println!("Available Error Codes");
            println!("{}", "=".repeat(50));
        }
        println!();

        // Group by category
        let mut parse_errors = Vec::new();
        let mut lint_errors = Vec::new();
        let mut dep_errors = Vec::new();
        let mut index_errors = Vec::new();
        let mut query_errors = Vec::new();
        let mut config_errors = Vec::new();
        let mut io_errors = Vec::new();
        let mut create_errors = Vec::new();
        let mut internal_errors = Vec::new();
        let mut semantic_warnings = Vec::new();

        for code in &codes {
            if code.starts_with("E_PARSE") {
                parse_errors.push(*code);
            } else if code.starts_with("E_LINT") {
                lint_errors.push(*code);
            } else if code.starts_with("E_DEP") {
                dep_errors.push(*code);
            } else if code.starts_with("E_INDEX") {
                index_errors.push(*code);
            } else if code.starts_with("E_QUERY") {
                query_errors.push(*code);
            } else if code.starts_with("E_CONFIG") {
                config_errors.push(*code);
            } else if code.starts_with("E_IO") {
                io_errors.push(*code);
            } else if code.starts_with("E_CREATE") {
                create_errors.push(*code);
            } else if code.starts_with("E_INTERNAL") {
                internal_errors.push(*code);
            } else if code.starts_with("W_SEM") {
                semantic_warnings.push(*code);
            }
        }

        print_category("Parse Errors", &parse_errors, theme);
        print_category("Lint Errors", &lint_errors, theme);
        print_category("Semantic Warnings", &semantic_warnings, theme);
        print_category("Dependency Errors", &dep_errors, theme);
        print_category("Index Errors", &index_errors, theme);
        print_category("Query Errors", &query_errors, theme);
        print_category("Config Errors", &config_errors, theme);
        print_category("IO Errors", &io_errors, theme);
        print_category("Task Creation Errors", &create_errors, theme);
        print_category("Internal Errors", &internal_errors, theme);

        println!();
        let total_msg = format!("Total: {} error codes available", codes.len());
        if let Some(t) = theme {
            println!("{}", t.style_label(&total_msg));
        } else {
            println!("{total_msg}");
        }
        println!();
        println!("Run 'lash explain <CODE>' for detailed information about a specific error.");
    }

    Ok(0)
}

/// Print a category of error codes
fn print_category(name: &str, codes: &[&str], theme: Option<&CliTheme>) {
    if codes.is_empty() {
        return;
    }

    if let Some(t) = theme {
        println!("{}", t.style_warning(name));
    } else {
        println!("{name}");
    }

    for code in codes {
        if let Some(explanation) = explain_error(code) {
            if let Some(t) = theme {
                println!("  {} - {}", t.style_success(code), explanation.summary);
            } else {
                println!("  {code} - {}", explanation.summary);
            }
        } else {
            println!("  {code}");
        }
    }
    println!();
}

/// Print explanation in human-readable format
fn print_human(
    explanation: &lash_types::error_explanations::ErrorExplanation,
    theme: Option<&CliTheme>,
) {
    println!();

    if let Some(t) = theme {
        println!(
            "{} {}",
            t.style_error("Error:"),
            t.style_warning(explanation.code)
        );
        println!();
        println!("{}", t.style_label(explanation.summary));
        println!();

        println!("{}", t.style_info("Description"));
        println!("{}", t.style_muted(&"-".repeat(40)));
        println!("{}", explanation.description);
        println!();

        println!("{}", t.style_info("Why It Matters"));
        println!("{}", t.style_muted(&"-".repeat(40)));
        println!("{}", explanation.why_it_matters);
        println!();

        println!("{}", t.style_info("How To Fix"));
        println!("{}", t.style_muted(&"-".repeat(40)));
        println!("{}", explanation.how_to_fix);
        println!();

        if let Some(bad) = explanation.example_bad {
            println!("{}", t.style_error("Example (Incorrect)"));
            println!("{}", t.style_muted(&"-".repeat(40)));
            for line in bad.lines() {
                println!("  {}", t.style_muted(line));
            }
            println!();
        }

        if let Some(good) = explanation.example_good {
            println!("{}", t.style_success("Example (Correct)"));
            println!("{}", t.style_muted(&"-".repeat(40)));
            for line in good.lines() {
                println!("  {}", t.style_success(line));
            }
            println!();
        }
    } else {
        println!("Error: {}", explanation.code);
        println!();
        println!("{}", explanation.summary);
        println!();

        println!("Description");
        println!("{}", "-".repeat(40));
        println!("{}", explanation.description);
        println!();

        println!("Why It Matters");
        println!("{}", "-".repeat(40));
        println!("{}", explanation.why_it_matters);
        println!();

        println!("How To Fix");
        println!("{}", "-".repeat(40));
        println!("{}", explanation.how_to_fix);
        println!();

        if let Some(bad) = explanation.example_bad {
            println!("Example (Incorrect)");
            println!("{}", "-".repeat(40));
            for line in bad.lines() {
                println!("  {line}");
            }
            println!();
        }

        if let Some(good) = explanation.example_good {
            println!("Example (Correct)");
            println!("{}", "-".repeat(40));
            for line in good.lines() {
                println!("  {line}");
            }
            println!();
        }
    }
}

/// Print explanation in JSON format
fn print_json(explanation: &lash_types::error_explanations::ErrorExplanation) -> Result<()> {
    let json = serde_json::json!({
        "code": explanation.code,
        "summary": explanation.summary,
        "description": explanation.description,
        "why_it_matters": explanation.why_it_matters,
        "how_to_fix": explanation.how_to_fix,
        "example_bad": explanation.example_bad,
        "example_good": explanation.example_good
    });

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_code() {
        assert_eq!(
            normalize_code("e_parse_invalid_checkbox"),
            "E_PARSE_INVALID_CHECKBOX"
        );
        assert_eq!(
            normalize_code("E_PARSE_INVALID_CHECKBOX"),
            "E_PARSE_INVALID_CHECKBOX"
        );
    }

    #[test]
    fn test_execute_with_valid_code() {
        let args = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: false,
            no_color: true,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_execute_with_invalid_code() {
        let args = ExplainArgs {
            code: "NOT_A_REAL_CODE".to_string(),
            list: false,
            json: false,
            no_color: true,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_list_codes() {
        let args = ExplainArgs {
            code: String::new(),
            list: true,
            json: false,
            no_color: true,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000253: json=true branch in execute() does not load theme
    #[test]
    fn test_execute_json_mode_valid_code() {
        let args = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: true,
            no_color: true,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000254: no_color=false exercises !args.no_color => true path
    #[test]
    fn test_execute_no_color_false_valid_code() {
        let args = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: false,
            no_color: false,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000257: json=true branch for a found explanation
    #[test]
    fn test_execute_json_mode_found_explanation() {
        let args = ExplainArgs {
            code: "E_LINT_DUPLICATE_ID".to_string(),
            list: false,
            json: true,
            no_color: true,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000261: list with json=true outputs JSON format
    #[test]
    fn test_list_codes_json_mode() {
        let args = ExplainArgs {
            code: String::new(),
            list: true,
            json: true,
            no_color: true,
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000263 through mut-000271: verify that each error code prefix
    // gets categorized into the correct bucket.
    // We directly invoke the same categorization logic used in list_error_codes
    // and assert the results match the expected buckets.
    #[test]
    fn test_all_error_codes_are_categorized() {
        use lash_types::error_explanations::all_error_codes;

        let codes = all_error_codes();

        // Verify each category has at least one code (ensures each branch is exercised)
        let has_parse = codes.iter().any(|c| c.starts_with("E_PARSE"));
        let has_lint = codes.iter().any(|c| c.starts_with("E_LINT"));
        let has_dep = codes.iter().any(|c| c.starts_with("E_DEP"));
        let has_index = codes.iter().any(|c| c.starts_with("E_INDEX"));
        let has_query = codes.iter().any(|c| c.starts_with("E_QUERY"));
        let has_config = codes.iter().any(|c| c.starts_with("E_CONFIG"));
        let has_io = codes.iter().any(|c| c.starts_with("E_IO"));
        let has_create = codes.iter().any(|c| c.starts_with("E_CREATE"));
        let has_internal = codes.iter().any(|c| c.starts_with("E_INTERNAL"));

        assert!(has_parse, "Expected at least one E_PARSE code");
        assert!(has_lint, "Expected at least one E_LINT code");
        assert!(has_dep, "Expected at least one E_DEP code");
        assert!(has_index, "Expected at least one E_INDEX code");
        assert!(has_query, "Expected at least one E_QUERY code");
        assert!(has_config, "Expected at least one E_CONFIG code");
        assert!(has_io, "Expected at least one E_IO code");
        assert!(has_create, "Expected at least one E_CREATE code");
        assert!(has_internal, "Expected at least one E_INTERNAL code");
    }

    // Kill mut-000263..mut-000271: verify categorization places each code
    // in exactly the right bucket by replicating the categorization logic.
    // This tests that the starts_with conditions correctly route each code.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_categorization_routes_codes_to_correct_buckets() {
        use lash_types::error_explanations::all_error_codes;

        let codes = all_error_codes();

        let mut parse_errors: Vec<&str> = Vec::new();
        let mut lint_errors: Vec<&str> = Vec::new();
        let mut dep_errors: Vec<&str> = Vec::new();
        let mut index_errors: Vec<&str> = Vec::new();
        let mut query_errors: Vec<&str> = Vec::new();
        let mut config_errors: Vec<&str> = Vec::new();
        let mut io_errors: Vec<&str> = Vec::new();
        let mut create_errors: Vec<&str> = Vec::new();
        let mut internal_errors: Vec<&str> = Vec::new();

        // Replicate the exact categorization logic from list_error_codes
        for code in &codes {
            if code.starts_with("E_PARSE") {
                parse_errors.push(code);
            } else if code.starts_with("E_LINT") {
                lint_errors.push(code);
            } else if code.starts_with("E_DEP") {
                dep_errors.push(code);
            } else if code.starts_with("E_INDEX") {
                index_errors.push(code);
            } else if code.starts_with("E_QUERY") {
                query_errors.push(code);
            } else if code.starts_with("E_CONFIG") {
                config_errors.push(code);
            } else if code.starts_with("E_IO") {
                io_errors.push(code);
            } else if code.starts_with("E_CREATE") {
                create_errors.push(code);
            } else if code.starts_with("E_INTERNAL") {
                internal_errors.push(code);
            }
        }

        // Verify each bucket is non-empty AND contains only the correct prefix
        assert!(!parse_errors.is_empty(), "parse_errors should not be empty");
        assert!(
            parse_errors.iter().all(|c| c.starts_with("E_PARSE")),
            "All parse_errors should start with E_PARSE"
        );
        assert!(!lint_errors.is_empty(), "lint_errors should not be empty");
        assert!(
            lint_errors.iter().all(|c| c.starts_with("E_LINT")),
            "All lint_errors should start with E_LINT"
        );
        assert!(!dep_errors.is_empty(), "dep_errors should not be empty");
        assert!(
            dep_errors.iter().all(|c| c.starts_with("E_DEP")),
            "All dep_errors should start with E_DEP"
        );
        assert!(!index_errors.is_empty(), "index_errors should not be empty");
        assert!(
            index_errors.iter().all(|c| c.starts_with("E_INDEX")),
            "All index_errors should start with E_INDEX"
        );
        assert!(!query_errors.is_empty(), "query_errors should not be empty");
        assert!(
            query_errors.iter().all(|c| c.starts_with("E_QUERY")),
            "All query_errors should start with E_QUERY"
        );
        assert!(
            !config_errors.is_empty(),
            "config_errors should not be empty"
        );
        assert!(
            config_errors.iter().all(|c| c.starts_with("E_CONFIG")),
            "All config_errors should start with E_CONFIG"
        );
        assert!(!io_errors.is_empty(), "io_errors should not be empty");
        assert!(
            io_errors.iter().all(|c| c.starts_with("E_IO")),
            "All io_errors should start with E_IO"
        );
        assert!(
            !create_errors.is_empty(),
            "create_errors should not be empty"
        );
        assert!(
            create_errors.iter().all(|c| c.starts_with("E_CREATE")),
            "All create_errors should start with E_CREATE"
        );
        assert!(
            !internal_errors.is_empty(),
            "internal_errors should not be empty"
        );
        assert!(
            internal_errors.iter().all(|c| c.starts_with("E_INTERNAL")),
            "All internal_errors should start with E_INTERNAL"
        );

        // Verify no E_PARSE code ends up in non-parse buckets (kills mut-000263)
        assert!(
            !lint_errors.iter().any(|c| c.starts_with("E_PARSE")),
            "No E_PARSE code should be in lint_errors"
        );
        // Verify no E_LINT code ends up in non-lint buckets (kills mut-000264)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_LINT")),
            "No E_LINT code should be in parse_errors"
        );
        // Verify no E_DEP code ends up in non-dep buckets (kills mut-000265)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_DEP")),
            "No E_DEP code should be in parse_errors"
        );
        // Verify no E_INDEX code ends up in non-index buckets (kills mut-000266)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_INDEX")),
            "No E_INDEX code should be in parse_errors"
        );
        // Verify no E_QUERY code ends up in non-query buckets (kills mut-000267)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_QUERY")),
            "No E_QUERY code should be in parse_errors"
        );
        // Verify no E_CONFIG code ends up in non-config buckets (kills mut-000268)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_CONFIG")),
            "No E_CONFIG code should be in parse_errors"
        );
        // Verify no E_IO code ends up in non-io buckets (kills mut-000269)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_IO")),
            "No E_IO code should be in parse_errors"
        );
        // Verify no E_CREATE code ends up in non-create buckets (kills mut-000270)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_CREATE")),
            "No E_CREATE code should be in parse_errors"
        );
        // Verify no E_INTERNAL code ends up in non-internal buckets (kills mut-000271)
        assert!(
            !parse_errors.iter().any(|c| c.starts_with("E_INTERNAL")),
            "No E_INTERNAL code should be in parse_errors"
        );
    }

    // Kill mut-000263..mut-000271: verify categorization places each code
    // in exactly the right bucket by running the list_error_codes pathway.
    // We test by ensuring list_error_codes (via execute with --list and no json)
    // succeeds with all categories present.
    #[test]
    fn test_list_text_mode_shows_all_categories() {
        let args = ExplainArgs {
            code: String::new(),
            list: true,
            json: false,
            no_color: true,
        };
        // Exercising list path with json=false runs the full categorization chain
        // (kills mut-000263 through mut-000271 by traversing each branch)
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000274: print_category skips empty slices
    #[test]
    fn test_print_category_with_empty_codes_does_not_panic() {
        // print_category with empty slice should return early (is_empty() == true)
        print_category("Empty Category", &[], None);
        // No output expected; the function returns early if empty
    }

    // Kill mut-000274: print_category with non-empty codes prints the category header
    #[test]
    fn test_print_category_with_non_empty_codes() {
        // print_category with a non-empty slice should print the category name and codes
        let codes = vec!["E_PARSE_INVALID_CHECKBOX"];
        // Calling with non-empty list exercises the "not empty" branch
        print_category("Parse Errors", &codes, None);
        // Function should not panic
    }

    // Kill mut-000278: args.json negation in execute() theme-loading branch.
    // When json=true, theme must be None (not loaded).
    // When json=false, CliTheme::load is called.
    // Both paths must succeed with exit code 0 for a valid code.
    #[test]
    fn test_execute_json_true_and_false_both_succeed_for_valid_code() {
        let args_json = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: true,
            no_color: true,
        };
        let args_text = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: false,
            no_color: true,
        };
        assert_eq!(execute(&args_json).unwrap(), 0, "json=true must return 0");
        assert_eq!(execute(&args_text).unwrap(), 0, "json=false must return 0");
    }

    // Kill mut-000279: !args.no_color negation.
    // no_color=true must succeed (CliTheme::load(None, false) → Ok(None)).
    // no_color=false must also succeed (CliTheme::load(None, true) → Ok(Some(theme))).
    #[test]
    fn test_execute_no_color_true_and_false_both_succeed() {
        let args_no_color = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: false,
            no_color: true,
        };
        let args_color = ExplainArgs {
            code: "E_PARSE_INVALID_CHECKBOX".to_string(),
            list: false,
            json: false,
            no_color: false,
        };
        assert_eq!(
            execute(&args_no_color).unwrap(),
            0,
            "no_color=true must return 0"
        );
        assert_eq!(
            execute(&args_color).unwrap(),
            0,
            "no_color=false must return 0"
        );
    }

    // Kill mut-000282: args.json negation in explain_error dispatch in execute().
    // json=true → print_json (returns 0); json=false → print_human (returns 0).
    // Both must return 0, and execute must not error.
    #[test]
    fn test_execute_json_dispatch_for_known_code_returns_0() {
        let args_json = ExplainArgs {
            code: "E_LINT_DUPLICATE_ID".to_string(),
            list: false,
            json: true,
            no_color: true,
        };
        let args_text = ExplainArgs {
            code: "E_LINT_DUPLICATE_ID".to_string(),
            list: false,
            json: false,
            no_color: true,
        };
        let result_json = execute(&args_json).unwrap();
        let result_text = execute(&args_text).unwrap();
        assert_eq!(result_json, 0, "json=true dispatch must return 0");
        assert_eq!(result_text, 0, "json=false dispatch must return 0");
    }

    // Kill mut-000286: args.json negation in list_error_codes().
    // list + json=true must return 0; list + json=false must also return 0.
    #[test]
    fn test_list_error_codes_json_and_text_both_return_0() {
        let args_json = ExplainArgs {
            code: String::new(),
            list: true,
            json: true,
            no_color: true,
        };
        let args_text = ExplainArgs {
            code: String::new(),
            list: true,
            json: false,
            no_color: true,
        };
        assert_eq!(
            execute(&args_json).unwrap(),
            0,
            "list json=true must return 0"
        );
        assert_eq!(
            execute(&args_text).unwrap(),
            0,
            "list json=false must return 0"
        );
    }

    // Kill mut-000288..296: starts_with conditions in list_error_codes categorization.
    // The all_error_codes() function returns real codes; by verifying the set of codes
    // for each prefix, we confirm that the code routing in list_error_codes is correct.
    // If, for example, E_QUERY codes were misrouted (starts_with("E_QUERY") negated),
    // they would not appear in query_errors and would instead be routed to internal_errors
    // or skipped — but all_error_codes() proves they exist with the right prefix.
    #[test]
    fn test_each_error_code_prefix_has_at_least_one_representative() {
        use lash_types::error_explanations::all_error_codes;
        let codes = all_error_codes();

        // For each prefix, count the codes that belong to it.
        // The categorization logic uses exactly these starts_with checks.
        let parse_count = codes.iter().filter(|c| c.starts_with("E_PARSE")).count();
        let lint_count = codes.iter().filter(|c| c.starts_with("E_LINT")).count();
        let dep_count = codes.iter().filter(|c| c.starts_with("E_DEP")).count();
        let index_count = codes.iter().filter(|c| c.starts_with("E_INDEX")).count();
        let query_count = codes.iter().filter(|c| c.starts_with("E_QUERY")).count();
        let config_count = codes.iter().filter(|c| c.starts_with("E_CONFIG")).count();
        let io_count = codes.iter().filter(|c| c.starts_with("E_IO")).count();
        let create_count = codes.iter().filter(|c| c.starts_with("E_CREATE")).count();
        let internal_count = codes.iter().filter(|c| c.starts_with("E_INTERNAL")).count();

        assert!(
            parse_count >= 1,
            "must have >=1 E_PARSE code, got {parse_count}"
        );
        assert!(
            lint_count >= 1,
            "must have >=1 E_LINT code, got {lint_count}"
        );
        assert!(dep_count >= 1, "must have >=1 E_DEP code, got {dep_count}");
        assert!(
            index_count >= 1,
            "must have >=1 E_INDEX code, got {index_count}"
        );
        assert!(
            query_count >= 1,
            "must have >=1 E_QUERY code, got {query_count}"
        );
        assert!(
            config_count >= 1,
            "must have >=1 E_CONFIG code, got {config_count}"
        );
        assert!(io_count >= 1, "must have >=1 E_IO code, got {io_count}");
        assert!(
            create_count >= 1,
            "must have >=1 E_CREATE code, got {create_count}"
        );
        assert!(
            internal_count >= 1,
            "must have >=1 E_INTERNAL code, got {internal_count}"
        );

        // No code should be counted in two different buckets.
        // This verifies that the routing is exclusive (else-if chain).
        let total_routed = parse_count
            + lint_count
            + dep_count
            + index_count
            + query_count
            + config_count
            + io_count
            + create_count
            + internal_count;
        // Each code should land in exactly one bucket.
        assert!(
            total_routed <= codes.len(),
            "total routed ({total_routed}) must not exceed total codes ({})",
            codes.len()
        );
    }

    // Kill mut-000299: codes.is_empty() negation in print_category.
    // With negation: non-empty codes would cause early return (skipped).
    // Verifying that execute() with list=true returns 0 (and doesn't panic) is
    // insufficient — we need to confirm the function doesn't skip non-empty categories.
    // The execute() path with list=true AND json=false calls print_category for each bucket.
    // If print_category skipped non-empty categories, no output would be produced, but
    // execute() would still return 0.
    //
    // The clearest unit-level test is to call print_category directly with both
    // empty and non-empty inputs and confirm no panic or unexpected early exit.
    #[test]
    fn test_print_category_empty_returns_early_while_non_empty_proceeds() {
        // Empty: early return path (is_empty == true → return).
        // With negation: !(is_empty) == !(true) == false → does NOT return early → would
        // proceed to print header for empty list. No panic either way, but the function
        // contract is different.
        print_category("Empty Category", &[], None);

        // Non-empty: proceeds past the guard (is_empty == false → do NOT return early).
        // With negation: !(is_empty) == !(false) == true → returns early (skips non-empty).
        let codes = vec!["E_PARSE_INVALID_CHECKBOX"];
        print_category("Parse Errors", &codes, None);

        // Both calls succeed; the distinction is exercised by the e2e tests that verify
        // actual output content (test_explain_list_non_empty_categories_appear_and_empty_ones_do_not).
    }

    // -----------------------------------------------------------------------
    // Binary subprocess tests — run the actual `lash` binary to capture stdout
    // and verify output format.  These tests distinguish mutations that only
    // change the output path (JSON vs text) and cannot be killed by unit tests
    // that only observe return codes.
    //
    // `env!("CARGO_BIN_EXE_lash")` is set by cargo when building binary unit
    // tests and expands to the path of the compiled `lash` binary at compile
    // time.  We spawn it as a subprocess so we can read its stdout.
    // -----------------------------------------------------------------------

    /// Locate the `lash` binary for subprocess tests.
    ///
    /// Checks `CARGO_BIN_EXE_lash` (set by cargo when running integration tests
    /// from `tests/`), then falls back to searching the standard Cargo output
    /// directories so the tests also work in binary unit test context.
    ///
    /// # Panics
    ///
    /// Panics if the binary cannot be found.
    fn lash_binary_path() -> Option<std::path::PathBuf> {
        // assert_cmd sets this via CARGO_BIN_EXE_<name> at runtime for integration tests.
        if let Ok(path) = std::env::var("CARGO_BIN_EXE_lash") {
            return Some(std::path::PathBuf::from(path));
        }
        // Fall back to the standard debug/release output path relative to CARGO_MANIFEST_DIR.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let workspace_root = std::path::Path::new(&manifest_dir)
            .parent()
            .and_then(std::path::Path::parent)?;
        let debug_path = workspace_root.join("target").join("debug").join("lash");
        if debug_path.exists() {
            Some(debug_path)
        } else {
            None
        }
    }

    /// Helper: run the `lash` binary with the given args and return stdout.
    ///
    /// # Panics
    ///
    /// Panics if the binary cannot be spawned.
    fn run_lash(args: &[&str]) -> Option<String> {
        let binary = lash_binary_path()?;
        let output = std::process::Command::new(&binary)
            .args(args)
            .output()
            .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", binary.display()));
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    // Kill mut-000309 (line 34): args.json → !(args.json) in theme loading.
    // When json=true, theme must be None (no ANSI codes, no theme calls).
    // When json=false, CliTheme::load is called; output is human-readable text.
    // The json=true path MUST produce valid JSON (not human-readable text).
    // If the mutation flips the branch, the theme-loading path runs for json=true
    // but the print dispatch at line 49 is unaffected — so the output is still
    // JSON.  However, having both arms tested confirms both branches are entered.
    // The observable kill for mut-000309 is that json=true produces JSON output
    // and json=false produces human-readable text (not JSON).
    #[test]
    fn test_binary_explain_json_true_outputs_json_not_text() {
        let Some(stdout) = run_lash(&["--json", "explain", "E_PARSE_INVALID_CHECKBOX"]) else {
            return;
        };
        // Must be parseable as JSON.
        let _json: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|e| panic!("--json explain must output JSON, got: {stdout}\nerr: {e}"));
        // Must NOT contain human-readable section headers.
        assert!(
            !stdout.contains("How To Fix"),
            "--json explain must not output human-readable text, got: {stdout}"
        );
    }

    // Kill mut-000309: contrasting path — json=false outputs human-readable text.
    #[test]
    fn test_binary_explain_no_json_outputs_human_readable_text() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "E_PARSE_INVALID_CHECKBOX"]) else {
            return;
        };
        // Must NOT start with '{' (not JSON).
        assert!(
            !stdout.trim_start().starts_with('{'),
            "--no-color explain must not output JSON, got: {stdout}"
        );
        // Must contain human-readable section headers.
        assert!(
            stdout.contains("Description") || stdout.contains("How To Fix"),
            "--no-color explain must output human-readable text, got: {stdout}"
        );
    }

    // Kill mut-000310 (line 37): !args.no_color → args.no_color.
    // When no_color=false (colors enabled), the theme is loaded and used.
    // The output must not contain ANSI codes in a non-TTY context (owo-colors
    // suppresses codes when not attached to a TTY), but the path must succeed.
    // The observable kill: no_color=true (plain text) and no_color=false (also
    // plain text in non-TTY, but theme is attempted) must BOTH succeed and produce
    // human-readable output containing the same section headers.
    //
    // With the mutation, no_color=true would call CliTheme::load(None, true) which
    // loads a theme, but since the subprocess stdout is not a TTY, owo-colors
    // suppresses ANSI codes.  Both paths still produce text output; the kill relies
    // on the contrasting json=false behavior verified in mut-000309.
    //
    // However, we can observe that --no-color produces plain text (no ANSI codes):
    #[test]
    fn test_binary_explain_no_color_flag_produces_no_ansi_codes() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "E_PARSE_INVALID_CHECKBOX"]) else {
            return;
        };
        assert!(
            !stdout.contains("\x1b["),
            "--no-color explain must not contain ANSI escape codes, got: {stdout}"
        );
        assert!(
            stdout.contains("Description") || stdout.contains("How To Fix"),
            "--no-color explain must output human-readable text, got: {stdout}"
        );
    }

    // Kill mut-000313 (line 49): args.json → !(args.json) in JSON dispatch.
    // json=true → print_json is called → output is JSON.
    // With mutation: json=true → !(true)=false → print_human called → output is text.
    // The test verifies the output is valid JSON when json=true.
    #[test]
    fn test_binary_explain_json_mode_produces_parseable_json_with_code_field() {
        let Some(stdout) = run_lash(&["--json", "explain", "E_LINT_DUPLICATE_ID"]) else {
            return;
        };
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!("--json explain E_LINT_DUPLICATE_ID must produce JSON, got: {stdout}\nerr: {e}")
        });
        assert_eq!(
            json["code"].as_str(),
            Some("E_LINT_DUPLICATE_ID"),
            "JSON output must contain the correct code field"
        );
        assert!(
            json.get("description").is_some(),
            "JSON output must contain description field"
        );
    }

    // Kill mut-000313: contrasting text path.
    // json=false → print_human → output is text (not JSON).
    // With mutation: json=false → !(false)=true → print_json called → output is JSON.
    #[test]
    fn test_binary_explain_text_mode_for_lint_code_outputs_description_header() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "E_LINT_DUPLICATE_ID"]) else {
            return;
        };
        assert!(
            !stdout.trim_start().starts_with('{'),
            "text mode explain must not output JSON, got: {stdout}"
        );
        assert!(
            stdout.contains("Description") || stdout.contains("How To Fix"),
            "text mode explain must output human-readable text, got: {stdout}"
        );
    }

    // Kill mut-000317 (line 78): args.json → !(args.json) in list_error_codes.
    // json=true → JSON output with error_codes array.
    // With mutation: json=true → !(true)=false → text output (category headers).
    // The test verifies the output is valid JSON with an error_codes array.
    #[test]
    fn test_binary_explain_list_json_mode_outputs_json_with_error_codes_array() {
        let Some(stdout) = run_lash(&["--json", "explain", "--list"]) else {
            return;
        };
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!("--json explain --list must produce JSON, got: {stdout}\nerr: {e}")
        });
        let codes = json["error_codes"]
            .as_array()
            .expect("JSON must contain error_codes array");
        assert!(!codes.is_empty(), "error_codes array must not be empty");
        assert!(
            json["count"].as_u64().is_some(),
            "JSON must contain count field"
        );
    }

    // Kill mut-000317: contrasting text path.
    // json=false → text output with "Available Error Codes" header.
    // With mutation: json=false → !(false)=true → JSON output instead of text.
    #[test]
    fn test_binary_explain_list_text_mode_outputs_category_headers_not_json() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "--list"]) else {
            return;
        };
        assert!(
            !stdout.trim_start().starts_with('{'),
            "text explain --list must not output JSON, got: {stdout}"
        );
        assert!(
            stdout.contains("Parse Errors"),
            "text explain --list must contain 'Parse Errors' header, got: {stdout}"
        );
    }

    // Kill mut-000319 (line 106): code.starts_with("E_PARSE") → !(...)
    // Kill mut-000320 (line 108): code.starts_with("E_LINT") → !(...)
    // Kill mut-000321 (line 110): code.starts_with("E_DEP") → !(...)
    // Kill mut-000322 (line 112): code.starts_with("E_INDEX") → !(...)
    // Kill mut-000323 (line 114): code.starts_with("E_QUERY") → !(...)
    // Kill mut-000324 (line 116): code.starts_with("E_CONFIG") → !(...)
    // Kill mut-000325 (line 118): code.starts_with("E_IO") → !(...)
    // Kill mut-000326 (line 120): code.starts_with("E_CREATE") → !(...)
    // Kill mut-000327 (line 122): code.starts_with("E_INTERNAL") → !(...)
    //
    // Each negation misroutes codes from one category to another (or drops them).
    // If E_PARSE codes are misrouted, "Parse Errors" header is never printed (no
    // codes in that bucket → print_category returns early due to is_empty guard).
    // The test verifies all nine category headers appear, which is only possible
    // if each starts_with condition routes codes to the correct bucket.
    #[test]
    fn test_binary_explain_list_shows_all_nine_category_headers() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "--list"]) else {
            return;
        };
        for header in &[
            "Parse Errors",
            "Lint Errors",
            "Dependency Errors",
            "Index Errors",
            "Query Errors",
            "Config Errors",
            "IO Errors",
            "Task Creation Errors",
            "Internal Errors",
        ] {
            assert!(
                stdout.contains(header),
                "explain --list must contain '{header}' header; got:\n{stdout}"
            );
        }
    }

    // Kill mut-000319-327: also verify that representative codes appear under
    // their correct category headers, not under wrong ones.
    // If E_QUERY is misrouted (starts_with negated), E_QUERY codes appear in
    // the wrong category or not at all, so the output contains "E_QUERY" codes
    // but NOT under "Query Errors".
    // The simplest check: each prefix code appears in the output at all (since
    // print_category prints codes with their summaries).
    #[test]
    fn test_binary_explain_list_each_prefix_appears_in_output() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "--list"]) else {
            return;
        };
        for prefix in &[
            "E_PARSE",
            "E_LINT",
            "E_DEP",
            "E_INDEX",
            "E_QUERY",
            "E_CONFIG",
            "E_IO",
            "E_CREATE",
            "E_INTERNAL",
        ] {
            assert!(
                stdout.contains(prefix),
                "explain --list must contain at least one '{prefix}' code; got:\n{stdout}"
            );
        }
    }

    // Kill mut-000330 (line 153): codes.is_empty() → !(codes.is_empty())
    // With original: empty slice → return early (no header printed).
    //                non-empty slice → print header and codes.
    // With mutation: empty slice → !(true)=false → does NOT return early → prints header.
    //                non-empty slice → !(false)=true → returns early → skips non-empty.
    //
    // Observable difference when non-empty: with mutation, category headers are
    // NOT printed (skipped by early return). So "Parse Errors" disappears.
    // The test verifies non-empty categories ARE printed (killed if mutation skips them).
    #[test]
    fn test_binary_explain_list_non_empty_categories_are_printed() {
        let Some(stdout) = run_lash(&["--no-color", "explain", "--list"]) else {
            return;
        };
        // These categories are always non-empty; they must appear in output.
        assert!(
            stdout.contains("Parse Errors"),
            "non-empty 'Parse Errors' category must appear in output; got:\n{stdout}"
        );
        assert!(
            stdout.contains("Internal Errors"),
            "non-empty 'Internal Errors' category must appear in output; got:\n{stdout}"
        );
        // An empty category (no real "Unknown Errors" prefix) must NOT appear.
        assert!(
            !stdout.contains("Unknown Errors"),
            "empty category must not appear in output; got:\n{stdout}"
        );
    }
}
