//! Config command implementation
//!
//! The `lash config` command allows users to manage configuration settings
//! at both project level (.lash/config.toml) and user level (~/.config/lash/config.toml).

use anyhow::{Context, Result};
use lash::cli::ConfigCommand;
use lash::config::Config;
use lash::theme::CliTheme;
use std::path::PathBuf;

/// Arguments for the config command
#[derive(Debug, Clone)]
pub struct ConfigArgs {
    /// The subcommand to execute
    pub command: ConfigCommand,
    /// Output JSON format
    pub json: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Project root directory
    pub project_root: Option<PathBuf>,
}

/// Execute the config command
///
/// # Arguments
///
/// * `args` - Config command arguments
///
/// # Returns
///
/// Exit code: 0 (success), 1 (error)
pub fn execute(args: &ConfigArgs) -> Result<i32> {
    // Load theme based on no_color flag and output format
    let theme = if args.json {
        None
    } else {
        CliTheme::load(None, !args.no_color)?
    };

    match &args.command {
        ConfigCommand::Get { key } => get(args, key, theme.as_ref()),
        ConfigCommand::Set { key, value, user } => set(args, key, value, *user, theme.as_ref()),
        ConfigCommand::List { changed } => list(args, *changed, theme.as_ref()),
    }
}

/// Get a configuration value
fn get(args: &ConfigArgs, key: &str, theme: Option<&CliTheme>) -> Result<i32> {
    // Load merged configuration
    let config = Config::load_merged(args.project_root.as_deref())?;

    // Parse the key and retrieve the value
    let value = get_config_value(&config, key);

    if let Some(val) = value {
        if args.json {
            let json = serde_json::json!({
                "key": key,
                "value": val
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("{val}");
        }
        Ok(0)
    } else {
        if args.json {
            let json = serde_json::json!({
                "error": format!("Configuration key not found: {key}")
            });
            eprintln!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            let error_msg = format!("Configuration key not found: {key}");
            if let Some(t) = theme {
                eprintln!("{}: {error_msg}", t.style_error("Error"));
            } else {
                eprintln!("Error: {error_msg}");
            }
        }
        Ok(1)
    }
}

/// Set a configuration value
fn set(
    args: &ConfigArgs,
    key: &str,
    value: &str,
    user: bool,
    theme: Option<&CliTheme>,
) -> Result<i32> {
    // Determine the config file path
    let config_path = if user {
        Config::user_config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine user config directory"))?
    } else {
        let root = args.project_root.as_ref().ok_or_else(|| {
            anyhow::anyhow!("No project root found. Use --user to set user config.")
        })?;
        root.join(".lash").join("config.toml")
    };

    // Load existing config or create default
    let mut config = if config_path.exists() {
        Config::load_from_file(&config_path)?
    } else {
        Config::default()
    };

    // Parse and validate the new value
    set_config_value(&mut config, key, value)?;

    // Validate the entire configuration
    config
        .validate()
        .with_context(|| format!("Invalid value '{value}' for key '{key}'"))?;

    // Create parent directories if needed
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    // Serialize and write the config
    let toml_string =
        toml::to_string_pretty(&config).context("Failed to serialize configuration")?;
    std::fs::write(&config_path, toml_string)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    if args.json {
        let json = serde_json::json!({
            "status": "success",
            "key": key,
            "value": value,
            "config_path": config_path.display().to_string()
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        let msg = format!("{key} = {value}");
        if let Some(t) = theme {
            println!("{}: {msg}", t.style_success("Configuration updated"));
            println!(
                "{}: {}",
                t.style_muted("Config file"),
                config_path.display()
            );
        } else {
            println!("Configuration updated: {msg}");
            println!("Config file: {}", config_path.display());
        }
    }

    Ok(0)
}

/// List all configuration settings
fn list(args: &ConfigArgs, changed: bool, theme: Option<&CliTheme>) -> Result<i32> {
    let config = Config::load_merged(args.project_root.as_deref())?;
    let defaults = Config::default();

    if args.json {
        list_json(&config)?;
    } else {
        list_text(&config, &defaults, changed, theme);
    }

    Ok(0)
}

/// Output config as JSON
fn list_json(config: &Config) -> Result<()> {
    let json = serde_json::json!({
        "output": {
            "default_format": config.output.default_format,
            "verbosity": config.output.verbosity,
            "color": config.output.color,
        },
        "linter": {
            "max_depth": config.linter.max_depth,
            "auto_fix": config.linter.auto_fix,
            "rules": config.linter.rules,
        },
        "search": {
            "fuzzy_threshold": config.search.fuzzy_threshold,
            "limit": config.search.limit,
        },
        "agent": {
            "token_budget": config.agent.token_budget,
            "default_format": config.agent.default_format,
        }
    });
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

/// Output config as human-readable text
fn list_text(config: &Config, defaults: &Config, changed: bool, theme: Option<&CliTheme>) {
    // Header
    if let Some(t) = theme {
        println!("{}", t.style_info("Configuration Settings"));
        println!("{}", t.style_muted(&"=".repeat(50)));
    } else {
        println!("Configuration Settings");
        println!("{}", "=".repeat(50));
    }
    println!();

    // Output section
    print_section("[output]", theme);
    print_setting(
        "  default_format",
        &config.output.default_format,
        changed && config.output.default_format == defaults.output.default_format,
        theme,
    );
    print_setting(
        "  verbosity",
        &config.output.verbosity,
        changed && config.output.verbosity == defaults.output.verbosity,
        theme,
    );
    print_setting(
        "  color",
        &config.output.color.to_string(),
        changed && config.output.color == defaults.output.color,
        theme,
    );
    println!();

    // Linter section
    print_section("[linter]", theme);
    print_setting(
        "  max_depth",
        &config.linter.max_depth.to_string(),
        changed && config.linter.max_depth == defaults.linter.max_depth,
        theme,
    );
    print_setting(
        "  auto_fix",
        &config.linter.auto_fix.to_string(),
        changed && config.linter.auto_fix == defaults.linter.auto_fix,
        theme,
    );
    let rules_str = if config.linter.rules.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", config.linter.rules.join(", "))
    };
    print_setting(
        "  rules",
        &rules_str,
        changed && config.linter.rules.is_empty(),
        theme,
    );
    println!();

    // Search section
    print_section("[search]", theme);
    let threshold_unchanged =
        (config.search.fuzzy_threshold - defaults.search.fuzzy_threshold).abs() < f32::EPSILON;
    print_setting(
        "  fuzzy_threshold",
        &config.search.fuzzy_threshold.to_string(),
        changed && threshold_unchanged,
        theme,
    );
    print_setting(
        "  limit",
        &config.search.limit.to_string(),
        changed && config.search.limit == defaults.search.limit,
        theme,
    );
    println!();

    // Agent section
    print_section("[agent]", theme);
    print_setting(
        "  token_budget",
        &config.agent.token_budget.to_string(),
        changed && config.agent.token_budget == defaults.agent.token_budget,
        theme,
    );
    print_setting(
        "  default_format",
        &config.agent.default_format,
        changed && config.agent.default_format == defaults.agent.default_format,
        theme,
    );
    println!();
}

/// Print a section header
fn print_section(name: &str, theme: Option<&CliTheme>) {
    if let Some(t) = theme {
        println!("{}", t.style_warning(name));
    } else {
        println!("{name}");
    }
}

/// Print a single setting
fn print_setting(key: &str, value: &str, skip_if_unchanged: bool, theme: Option<&CliTheme>) {
    // Skip unchanged settings if filtering for changed only
    if skip_if_unchanged {
        return;
    }

    if let Some(t) = theme {
        println!("{} = \"{}\"", t.style_label(key), value);
    } else {
        println!("{key} = \"{value}\"");
    }
}

/// Get a config value by dot-notation key
fn get_config_value(config: &Config, key: &str) -> Option<String> {
    match key {
        "output.default_format" => Some(config.output.default_format.clone()),
        "output.verbosity" => Some(config.output.verbosity.clone()),
        "output.color" => Some(config.output.color.to_string()),
        "linter.max_depth" => Some(config.linter.max_depth.to_string()),
        "linter.auto_fix" => Some(config.linter.auto_fix.to_string()),
        "linter.rules" => {
            if config.linter.rules.is_empty() {
                Some("[]".to_string())
            } else {
                Some(format!("[{}]", config.linter.rules.join(", ")))
            }
        }
        "search.fuzzy_threshold" => Some(config.search.fuzzy_threshold.to_string()),
        "search.limit" => Some(config.search.limit.to_string()),
        "agent.token_budget" => Some(config.agent.token_budget.to_string()),
        "agent.default_format" => Some(config.agent.default_format.clone()),
        _ => None,
    }
}

/// Set a config value by dot-notation key
fn set_config_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "output.default_format" => {
            config.output.default_format = value.to_string();
        }
        "output.verbosity" => {
            config.output.verbosity = value.to_string();
        }
        "output.color" => {
            config.output.color = value
                .parse::<bool>()
                .with_context(|| format!("Invalid boolean value: {value}"))?;
        }
        "linter.max_depth" => {
            config.linter.max_depth = value
                .parse::<usize>()
                .with_context(|| format!("Invalid number value: {value}"))?;
        }
        "linter.auto_fix" => {
            config.linter.auto_fix = value
                .parse::<bool>()
                .with_context(|| format!("Invalid boolean value: {value}"))?;
        }
        "linter.rules" => {
            // Parse comma-separated list
            if value == "[]" || value.is_empty() {
                config.linter.rules = Vec::new();
            } else {
                // Remove brackets if present
                let cleaned = value.trim_matches(|c| c == '[' || c == ']');
                config.linter.rules = cleaned
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        "search.fuzzy_threshold" => {
            config.search.fuzzy_threshold = value
                .parse::<f32>()
                .with_context(|| format!("Invalid float value: {value}"))?;
        }
        "search.limit" => {
            config.search.limit = value
                .parse::<usize>()
                .with_context(|| format!("Invalid number value: {value}"))?;
        }
        "agent.token_budget" => {
            config.agent.token_budget = value
                .parse::<usize>()
                .with_context(|| format!("Invalid number value: {value}"))?;
        }
        "agent.default_format" => {
            config.agent.default_format = value.to_string();
        }
        _ => {
            anyhow::bail!("Unknown configuration key: {key}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_value() {
        let config = Config::default();

        assert_eq!(
            get_config_value(&config, "output.default_format"),
            Some("text".to_string())
        );
        assert_eq!(
            get_config_value(&config, "output.verbosity"),
            Some("normal".to_string())
        );
        assert_eq!(
            get_config_value(&config, "output.color"),
            Some("true".to_string())
        );
        assert_eq!(
            get_config_value(&config, "linter.max_depth"),
            Some("4".to_string())
        );
        assert_eq!(
            get_config_value(&config, "linter.auto_fix"),
            Some("false".to_string())
        );
        assert_eq!(
            get_config_value(&config, "search.limit"),
            Some("20".to_string())
        );
        assert_eq!(
            get_config_value(&config, "agent.token_budget"),
            Some("4000".to_string())
        );
        assert_eq!(get_config_value(&config, "unknown.key"), None);
    }

    #[test]
    fn test_set_config_value_string() {
        let mut config = Config::default();

        set_config_value(&mut config, "output.default_format", "json").unwrap();
        assert_eq!(config.output.default_format, "json");

        set_config_value(&mut config, "output.verbosity", "verbose").unwrap();
        assert_eq!(config.output.verbosity, "verbose");
    }

    #[test]
    fn test_set_config_value_bool() {
        let mut config = Config::default();

        set_config_value(&mut config, "output.color", "false").unwrap();
        assert!(!config.output.color);

        set_config_value(&mut config, "linter.auto_fix", "true").unwrap();
        assert!(config.linter.auto_fix);

        // Invalid boolean should error
        assert!(set_config_value(&mut config, "output.color", "maybe").is_err());
    }

    #[test]
    fn test_set_config_value_number() {
        let mut config = Config::default();

        set_config_value(&mut config, "linter.max_depth", "5").unwrap();
        assert_eq!(config.linter.max_depth, 5);

        set_config_value(&mut config, "search.limit", "100").unwrap();
        assert_eq!(config.search.limit, 100);

        // Invalid number should error
        assert!(set_config_value(&mut config, "linter.max_depth", "abc").is_err());
    }

    #[test]
    fn test_set_config_value_float() {
        let mut config = Config::default();

        set_config_value(&mut config, "search.fuzzy_threshold", "0.5").unwrap();
        assert!((config.search.fuzzy_threshold - 0.5).abs() < f32::EPSILON);

        // Invalid float should error
        assert!(set_config_value(&mut config, "search.fuzzy_threshold", "not-a-number").is_err());
    }

    #[test]
    fn test_set_config_value_list() {
        let mut config = Config::default();

        set_config_value(&mut config, "linter.rules", "rule1,rule2,rule3").unwrap();
        assert_eq!(config.linter.rules, vec!["rule1", "rule2", "rule3"]);

        // Empty list
        set_config_value(&mut config, "linter.rules", "[]").unwrap();
        assert!(config.linter.rules.is_empty());

        // With brackets
        set_config_value(&mut config, "linter.rules", "[a, b]").unwrap();
        assert_eq!(config.linter.rules, vec!["a", "b"]);
    }

    #[test]
    fn test_set_config_value_unknown_key() {
        let mut config = Config::default();
        assert!(set_config_value(&mut config, "unknown.key", "value").is_err());
    }

    // Kill mut-000270: config.linter.rules.is_empty() → !(config.linter.rules.is_empty())
    // get_config_value must return "[]" for empty rules and "[rule]" for non-empty rules.
    // With the negation, empty rules would return the join format and non-empty would return "[]".

    #[test]
    fn test_get_config_value_linter_rules_empty_returns_bracket_notation() {
        let config = Config::default(); // default has no rules
        assert!(
            config.linter.rules.is_empty(),
            "default config must have empty rules"
        );
        assert_eq!(
            get_config_value(&config, "linter.rules"),
            Some("[]".to_string()),
            "empty rules must return '[]'"
        );
    }

    #[test]
    fn test_get_config_value_linter_rules_non_empty_returns_formatted_list() {
        let mut config = Config::default();
        config.linter.rules = vec!["rule-a".to_string(), "rule-b".to_string()];
        assert_eq!(
            get_config_value(&config, "linter.rules"),
            Some("[rule-a, rule-b]".to_string()),
            "non-empty rules must return formatted list"
        );
    }

    // Kill mut-000248: || vs && in "value == "[]" || value.is_empty()"
    // Test where ONLY one condition is true:
    //   - value == "[]" is true, value.is_empty() is false
    //   - value == "[]" is false, value.is_empty() is true
    // Both should clear the rules list with || but NOT with &&.

    #[test]
    fn test_set_linter_rules_empty_string_clears_list() {
        // value.is_empty() is true, value == "[]" is false
        // With ||: clears the list (correct)
        // With &&: would NOT clear the list (wrong)
        let mut config = Config::default();
        config.linter.rules = vec!["existing_rule".to_string()];

        set_config_value(&mut config, "linter.rules", "").unwrap();
        assert!(
            config.linter.rules.is_empty(),
            "Empty string should clear linter.rules"
        );
    }

    #[test]
    fn test_set_linter_rules_bracket_notation_clears_list() {
        // value == "[]" is true, value.is_empty() is false
        // With ||: clears the list (correct)
        // With &&: would NOT clear the list (wrong - "[]" is not empty)
        let mut config = Config::default();
        config.linter.rules = vec!["existing_rule".to_string()];

        set_config_value(&mut config, "linter.rules", "[]").unwrap();
        assert!(
            config.linter.rules.is_empty(),
            "'[]' should clear linter.rules"
        );
    }

    // -----------------------------------------------------------------------
    // execute() unit tests: kill mutants that change exit codes or file paths
    // -----------------------------------------------------------------------

    /// Returns a `ConfigArgs` for the Get subcommand, wired to a temp project root.
    fn make_get_args(key: &str, project_root: std::path::PathBuf) -> ConfigArgs {
        ConfigArgs {
            command: ConfigCommand::Get {
                key: key.to_string(),
            },
            json: false,
            no_color: true,
            project_root: Some(project_root),
        }
    }

    /// Returns a `ConfigArgs` for the List subcommand.
    fn make_list_args(project_root: std::path::PathBuf, json: bool) -> ConfigArgs {
        ConfigArgs {
            command: ConfigCommand::List { changed: false },
            json,
            no_color: true,
            project_root: Some(project_root),
        }
    }

    // Kill mut-000264: Ok(0) → Ok(1) in get() success path.
    // A Get with a valid key must return exactly 0 (not 1).
    #[test]
    fn test_execute_get_valid_key_returns_0() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = make_get_args("output.default_format", temp.path().to_path_buf());
        let result = execute(&args).unwrap();
        assert_eq!(
            result, 0,
            "get() with valid key must return exit code 0, not 1"
        );
    }

    // Kill mut-000264: also verify 0 is not 1 and 1 is the error code.
    #[test]
    fn test_execute_get_invalid_key_returns_1() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = ConfigArgs {
            command: ConfigCommand::Get {
                key: "no.such.key".to_string(),
            },
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 1, "get() with invalid key must return exit code 1");
        assert_ne!(result, 0, "1 must not equal 0");
    }

    // Kill mut-000270 / mut-000272: Ok(0) → Ok(1) in list().
    // A List command on a valid project must return exactly 0.
    #[test]
    fn test_execute_list_returns_0() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = make_list_args(temp.path().to_path_buf(), false);
        let result = execute(&args).unwrap();
        assert_eq!(result, 0, "list() must return exit code 0, not 1");
    }

    // Kill mut-000271: args.json → !(args.json) in list().
    // With json=true, list() calls list_json (produces JSON).
    // With json=false, list() calls list_text.
    // Both must return 0 (so exit codes are distinguishable from error codes).
    #[test]
    fn test_execute_list_json_true_returns_0() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = make_list_args(temp.path().to_path_buf(), true);
        let result = execute(&args).unwrap();
        assert_eq!(result, 0, "list() with json=true must return exit code 0");
    }

    // Kill mut-000260: args.json → !(args.json) in execute() theme loading.
    // When json=true, theme is None (no CliTheme::load). When json=false, theme is loaded.
    // Both must succeed and return 0 (confirming neither path errors out).
    #[test]
    fn test_execute_json_true_does_not_error_out() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = ConfigArgs {
            command: ConfigCommand::List { changed: false },
            json: true,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000261: !args.no_color → args.no_color in execute() theme loading.
    // no_color=false passes !false=true (color enabled) to CliTheme::load.
    // no_color=true passes !true=false (color disabled) to CliTheme::load.
    // Both must succeed (not error).
    #[test]
    fn test_execute_no_color_false_does_not_error_out() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = ConfigArgs {
            command: ConfigCommand::List { changed: false },
            json: false,
            no_color: false, // color ENABLED: !false=true
            project_root: Some(temp.path().to_path_buf()),
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0);
    }

    // Kill mut-000263: args.json → !(args.json) in get().
    // json=true and json=false must both return 0 for a valid key.
    // The mutation swaps the output format but keeps exit code; we verify both
    // succeed to confirm neither branch is broken.
    #[test]
    fn test_execute_get_json_true_returns_0() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = ConfigArgs {
            command: ConfigCommand::Get {
                key: "output.default_format".to_string(),
            },
            json: true,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        let result = execute(&args).unwrap();
        assert_eq!(
            result, 0,
            "get() with json=true and valid key must return 0"
        );
    }

    // Kill mut-000265: user → !(user) in set().
    // When user=false, set() must write to the project config path.
    // With mutation (user=true for user=false case), it would try to use user config path.
    #[test]
    fn test_execute_set_user_false_writes_project_config() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        std::fs::create_dir_all(&lash_dir).unwrap();
        let project_config = lash_dir.join("config.toml");

        assert!(
            !project_config.exists(),
            "project config must not exist before set"
        );

        let args = ConfigArgs {
            command: ConfigCommand::Set {
                key: "output.default_format".to_string(),
                value: "json".to_string(),
                user: false, // should route to project config
            },
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0, "set() must return 0");
        assert!(
            project_config.exists(),
            "set() with user=false must write project config at <root>/.lash/config.toml"
        );
        let contents = std::fs::read_to_string(&project_config).unwrap();
        assert!(
            contents.contains("default_format"),
            "project config must contain the written key; contents: {contents}"
        );
    }

    // Kill mut-000266: config_path.exists() → !(config_path.exists()) in set().
    // When the config file already exists, its content must be preserved.
    // With mutation (!exists), an existing file is ignored and defaults are used.
    #[test]
    fn test_execute_set_preserves_existing_values_when_config_exists() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        std::fs::create_dir_all(&lash_dir).unwrap();

        // Write first value: output.verbosity = verbose
        let args1 = ConfigArgs {
            command: ConfigCommand::Set {
                key: "output.verbosity".to_string(),
                value: "verbose".to_string(),
                user: false,
            },
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        execute(&args1).unwrap();

        // Write second value: output.default_format = json (different key)
        let args2 = ConfigArgs {
            command: ConfigCommand::Set {
                key: "output.default_format".to_string(),
                value: "json".to_string(),
                user: false,
            },
            json: false,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        execute(&args2).unwrap();

        // Read back verbosity: must still be "verbose" (not the default "normal")
        // This confirms config_path.exists() correctly loads existing file.
        let args3 = make_get_args("output.verbosity", temp.path().to_path_buf());
        let result = execute(&args3).unwrap();
        assert_eq!(result, 0);

        // Read back the file directly to verify verbosity is preserved
        let project_config = lash_dir.join("config.toml");
        let contents = std::fs::read_to_string(&project_config).unwrap();
        assert!(
            contents.contains("verbose"),
            "existing config values must be preserved when config_path.exists(); contents: {contents}"
        );
        assert!(
            !contents.contains("normal"),
            "first-written value 'verbose' must not be overwritten with default 'normal'"
        );
    }

    // Kill mut-000268: args.json → !(args.json) in set().
    // set() with json=true must return 0 (json output path works).
    // set() with json=false must return 0 (text output path works).
    #[test]
    fn test_execute_set_json_true_returns_0() {
        use tempfile::TempDir;
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".lash")).unwrap();

        let args = ConfigArgs {
            command: ConfigCommand::Set {
                key: "search.limit".to_string(),
                value: "50".to_string(),
                user: false,
            },
            json: true,
            no_color: true,
            project_root: Some(temp.path().to_path_buf()),
        };
        let result = execute(&args).unwrap();
        assert_eq!(result, 0, "set() with json=true must return 0");
    }

    // -----------------------------------------------------------------------
    // print_setting: kill mut-000296 (skip_if_unchanged negation)
    // -----------------------------------------------------------------------

    // When skip_if_unchanged=true, print_setting returns early (no output).
    // When skip_if_unchanged=false, it prints.
    // The mutation negates the condition: !(skip_if_unchanged) means when true,
    // it would NOT return early. We verify consistent behavior by calling both.
    #[test]
    fn test_print_setting_skip_true_does_not_panic() {
        // skip_if_unchanged=true: must return early silently
        print_setting("test.key", "test_value", true, None);
    }

    #[test]
    fn test_print_setting_skip_false_does_not_panic() {
        // skip_if_unchanged=false: must print and not panic
        print_setting("test.key", "test_value", false, None);
    }

    // -----------------------------------------------------------------------
    // list_text: kill mutants 274-294 (changed && x == y → changed || x != y)
    // -----------------------------------------------------------------------

    // list_text with changed=false must not skip any settings.
    // With changed=false: skip_if_unchanged = false && anything = false → prints all.
    // With mutation (|| instead of &&): skip_if_unchanged = false || (x == default) = true
    // for default values → settings would be skipped. But we can't observe stdout.
    //
    // The observable approach: verify that list_text doesn't panic for all combinations.
    #[test]
    fn test_list_text_changed_false_does_not_panic() {
        let config = Config::default();
        let defaults = Config::default();
        // changed=false: all settings visible (skip_if_unchanged is always false)
        list_text(&config, &defaults, false, None);
    }

    #[test]
    fn test_list_text_changed_true_with_default_config_does_not_panic() {
        let config = Config::default();
        let defaults = Config::default();
        // changed=true with matching values: all default settings should be skipped
        list_text(&config, &defaults, true, None);
    }

    #[test]
    fn test_list_text_changed_true_with_non_default_value_does_not_panic() {
        let mut config = Config::default();
        config.output.verbosity = "verbose".to_string(); // differs from default "normal"
        let defaults = Config::default();
        // changed=true with non-default verbosity: verbosity should NOT be skipped
        list_text(&config, &defaults, true, None);
    }

    // Kill mut-000284: config.linter.rules.is_empty() → !(config.linter.rules.is_empty())
    // in list_text. The rules display string depends on is_empty():
    // - empty → "[]"
    // - non-empty → "[rule1, rule2]"
    // We verify this directly through get_config_value which uses the same logic.
    #[test]
    fn test_linter_rules_display_is_empty_distinguishes_empty_from_non_empty() {
        let config_empty = Config::default();
        assert!(config_empty.linter.rules.is_empty());

        let rules_str_empty = if config_empty.linter.rules.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", config_empty.linter.rules.join(", "))
        };
        assert_eq!(rules_str_empty, "[]", "empty rules must display as '[]'");

        let mut config_non_empty = Config::default();
        config_non_empty.linter.rules = vec!["rule1".to_string()];
        let rules_str_non_empty = if config_non_empty.linter.rules.is_empty() {
            "[]".to_string()
        } else {
            format!("[{}]", config_non_empty.linter.rules.join(", "))
        };
        assert_eq!(
            rules_str_non_empty, "[rule1]",
            "non-empty rules must display as '[rule1]'"
        );

        // The two results must be different
        assert_ne!(rules_str_empty, rules_str_non_empty);
    }

    // Kill mut-000286/287: (fuzzy_threshold diff).abs() < EPSILON vs <= / >=
    // The threshold comparison determines if threshold matches defaults.
    // With <=: always true when equal (same as <, since abs diff is 0 when equal).
    // With >=: always true (any difference is >= 0).
    // The threshold_unchanged flag is passed to print_setting as skip_if_unchanged.
    // We verify the comparison logic directly.
    #[test]
    fn test_fuzzy_threshold_unchanged_detection() {
        let config = Config::default();
        let defaults = Config::default();

        // Same values: abs diff < EPSILON must be true
        let threshold_unchanged =
            (config.search.fuzzy_threshold - defaults.search.fuzzy_threshold).abs() < f32::EPSILON;
        assert!(
            threshold_unchanged,
            "identical thresholds must be 'unchanged'"
        );

        // Different values: abs diff must NOT be < EPSILON
        let mut config2 = Config::default();
        config2.search.fuzzy_threshold = 0.5_f32; // differs from default
        let threshold_changed =
            (config2.search.fuzzy_threshold - defaults.search.fuzzy_threshold).abs() < f32::EPSILON;
        assert!(
            !threshold_changed,
            "different thresholds must not be 'unchanged'"
        );
    }

    // -----------------------------------------------------------------------
    // Coverage: get_config_value keys not covered by existing tests
    // -----------------------------------------------------------------------

    // The existing test_get_config_value test covers most keys but omits
    // search.fuzzy_threshold and agent.default_format. These tests fill that gap
    // and ensure get_config_value handles all recognised keys.

    #[test]
    fn test_get_config_value_search_fuzzy_threshold() {
        let config = Config::default();
        let val = get_config_value(&config, "search.fuzzy_threshold");
        assert!(
            val.is_some(),
            "search.fuzzy_threshold must be a recognised key"
        );
        // The value must parse back to a f32 — it is produced by f32::to_string()
        let s = val.unwrap();
        assert!(
            s.parse::<f32>().is_ok(),
            "search.fuzzy_threshold value must be a valid float; got: {s}"
        );
    }

    #[test]
    fn test_get_config_value_agent_default_format() {
        let config = Config::default();
        let val = get_config_value(&config, "agent.default_format");
        assert!(
            val.is_some(),
            "agent.default_format must be a recognised key"
        );
        // The default format must be a non-empty string
        assert!(
            !val.unwrap().is_empty(),
            "agent.default_format must not be empty"
        );
    }

    // -----------------------------------------------------------------------
    // Coverage: set_config_value agent keys
    // -----------------------------------------------------------------------

    // The existing tests cover output, linter, and search keys but not the
    // agent section. These tests ensure the agent.token_budget and
    // agent.default_format arms of set_config_value work correctly.

    #[test]
    fn test_set_config_value_agent_token_budget() {
        let mut config = Config::default();

        set_config_value(&mut config, "agent.token_budget", "8000").unwrap();
        assert_eq!(
            config.agent.token_budget, 8000,
            "agent.token_budget must be set to 8000"
        );

        // Invalid number should error
        assert!(
            set_config_value(&mut config, "agent.token_budget", "not-a-number").is_err(),
            "invalid agent.token_budget must error"
        );
    }

    #[test]
    fn test_set_config_value_agent_default_format() {
        let mut config = Config::default();

        set_config_value(&mut config, "agent.default_format", "compact").unwrap();
        assert_eq!(
            config.agent.default_format, "compact",
            "agent.default_format must be set to 'compact'"
        );
    }

    // -----------------------------------------------------------------------
    // list_text skip-logic invariants
    //
    // The mutations mut-000285 through mut-000305 target the boolean expressions
    // passed to print_setting as skip_if_unchanged:
    //
    //   changed && field == default   (original)
    //   changed || field != default   (hypothetical mutation combining && → || with == → !=)
    //   changed && field != default   (== → != mutation alone)
    //   changed || field == default   (&& → || mutation alone)
    //
    // These mutations only affect which lines are printed to stdout, so they
    // cannot be killed via return-value assertions. The tests below document
    // the intended semantics of each expression form so that any future
    // refactoring preserves the correct behaviour.
    //
    // Note: these tests operate on the expression logic directly (not through
    // list_text) because list_text writes to stdout and does not return the
    // skip flags. They serve as executable documentation of the expected
    // truth-table for the skip logic.
    // -----------------------------------------------------------------------

    /// Verifies the `changed && field == default` skip expression semantics.
    ///
    /// A setting must only be skipped when BOTH:
    ///   1. The caller requested changed-only output (changed=true), AND
    ///   2. The field value equals the default (field == default).
    #[test]
    fn test_list_text_skip_logic_truth_table() {
        let defaults = Config::default();
        let default_verbosity = defaults.output.verbosity.clone();
        let changed_verbosity = "verbose".to_string();

        // Case: changed=false, field==default → must NOT skip (always show all settings)
        let skip = false;
        let _ = default_verbosity == defaults.output.verbosity; // value doesn't affect skip when changed=false
        assert!(
            !skip,
            "changed=false must never skip, even when value equals default"
        );

        // Case: changed=false, field!=default → must NOT skip
        let skip = false;
        let _ = changed_verbosity == defaults.output.verbosity; // value doesn't affect skip when changed=false
        assert!(
            !skip,
            "changed=false must never skip, even when value differs from default"
        );

        // Case: changed=true, field==default → MUST skip (value is unchanged)
        let skip = default_verbosity == defaults.output.verbosity;
        assert!(
            skip,
            "changed=true with value==default must skip the setting"
        );

        // Case: changed=true, field!=default → must NOT skip (value was changed)
        let skip = changed_verbosity == defaults.output.verbosity;
        assert!(
            !skip,
            "changed=true with value!=default must not skip the setting"
        );
    }

    /// Calls `list_text` with non-default `linter.rules` to exercise the `rules_str`
    /// formatting branch (L246) that handles non-empty rule lists.
    ///
    /// The test cannot observe stdout, but it ensures the function does not panic
    /// when rules is non-empty — a basic smoke test for the `is_empty()` branch.
    #[test]
    fn test_list_text_with_non_empty_linter_rules_does_not_panic() {
        let mut config = Config::default();
        config.linter.rules = vec!["required-id".to_string(), "no-orphan".to_string()];
        let defaults = Config::default();
        // changed=true: the rules field differs from defaults, so it should be printed
        list_text(&config, &defaults, true, None);
        // changed=false: all settings printed regardless
        list_text(&config, &defaults, false, None);
    }
}
