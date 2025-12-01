//! Explain command implementation
//!
//! The `lash explain` command provides detailed explanations of error codes,
//! including what causes them, why they matter, and how to fix them.

use anyhow::Result;
use lash_types::error_explanations::{all_error_codes, explain_error};
use owo_colors::OwoColorize;

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
    // Handle --list flag
    if args.list {
        return list_error_codes(args);
    }

    // Look up the error code
    let code = normalize_code(&args.code);

    if let Some(explanation) = explain_error(&code) {
        if args.json {
            print_json(&explanation)?;
        } else {
            print_human(&explanation, !args.no_color);
        }
        Ok(0)
    } else {
        if args.no_color {
            eprintln!("Error: Unknown error code '{}'", args.code);
        } else {
            eprintln!(
                "{}: Unknown error code '{}'",
                "Error".red().bold(),
                args.code
            );
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
fn list_error_codes(args: &ExplainArgs) -> Result<i32> {
    let codes = all_error_codes();

    if args.json {
        let json = serde_json::json!({
            "error_codes": codes,
            "count": codes.len()
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        let use_color = !args.no_color;

        if use_color {
            println!("{}", "Available Error Codes".cyan().bold());
            println!("{}", "=".repeat(50).dimmed());
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
        let mut internal_errors = Vec::new();

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
            } else if code.starts_with("E_INTERNAL") {
                internal_errors.push(*code);
            }
        }

        print_category("Parse Errors", &parse_errors, use_color);
        print_category("Lint Errors", &lint_errors, use_color);
        print_category("Dependency Errors", &dep_errors, use_color);
        print_category("Index Errors", &index_errors, use_color);
        print_category("Query Errors", &query_errors, use_color);
        print_category("Config Errors", &config_errors, use_color);
        print_category("IO Errors", &io_errors, use_color);
        print_category("Internal Errors", &internal_errors, use_color);

        println!();
        if use_color {
            println!("{} {} error codes available", "Total:".bold(), codes.len());
        } else {
            println!("Total: {} error codes available", codes.len());
        }
        println!();
        println!("Run 'lash explain <CODE>' for detailed information about a specific error.");
    }

    Ok(0)
}

/// Print a category of error codes
fn print_category(name: &str, codes: &[&str], use_color: bool) {
    if codes.is_empty() {
        return;
    }

    if use_color {
        println!("{}", name.yellow().bold());
    } else {
        println!("{name}");
    }

    for code in codes {
        if let Some(explanation) = explain_error(code) {
            if use_color {
                println!("  {} - {}", code.green(), explanation.summary);
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
fn print_human(explanation: &lash_types::error_explanations::ErrorExplanation, use_color: bool) {
    println!();

    if use_color {
        println!("{} {}", "Error:".red().bold(), explanation.code.yellow());
        println!();
        println!("{}", explanation.summary.white().bold());
        println!();

        println!("{}", "Description".cyan().bold());
        println!("{}", "-".repeat(40).dimmed());
        println!("{}", explanation.description);
        println!();

        println!("{}", "Why It Matters".cyan().bold());
        println!("{}", "-".repeat(40).dimmed());
        println!("{}", explanation.why_it_matters);
        println!();

        println!("{}", "How To Fix".cyan().bold());
        println!("{}", "-".repeat(40).dimmed());
        println!("{}", explanation.how_to_fix);
        println!();

        if let Some(bad) = explanation.example_bad {
            println!("{}", "Example (Incorrect)".red().bold());
            println!("{}", "-".repeat(40).dimmed());
            for line in bad.lines() {
                println!("  {}", line.dimmed());
            }
            println!();
        }

        if let Some(good) = explanation.example_good {
            println!("{}", "Example (Correct)".green().bold());
            println!("{}", "-".repeat(40).dimmed());
            for line in good.lines() {
                println!("  {}", line.green());
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
}
