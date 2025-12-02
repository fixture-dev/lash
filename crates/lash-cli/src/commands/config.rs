//! Config command implementation
//!
//! The `lash config` command allows users to manage configuration settings
//! at both project level (.lash/config.toml) and user level (~/.config/lash/config.toml).

use anyhow::{Context, Result};
use lash_cli::cli::ConfigCommand;
use lash_cli::config::Config;
use owo_colors::OwoColorize;
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
    match &args.command {
        ConfigCommand::Get { key } => get(args, key),
        ConfigCommand::Set { key, value, user } => set(args, key, value, *user),
        ConfigCommand::List { changed } => list(args, *changed),
    }
}

/// Get a configuration value
fn get(args: &ConfigArgs, key: &str) -> Result<i32> {
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
        } else if args.no_color {
            eprintln!("Error: Configuration key not found: {key}");
        } else {
            eprintln!(
                "{}: Configuration key not found: {key}",
                "Error".red().bold(),
            );
        }
        Ok(1)
    }
}

/// Set a configuration value
fn set(args: &ConfigArgs, key: &str, value: &str, user: bool) -> Result<i32> {
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
    } else if args.no_color {
        println!("Configuration updated: {key} = {value}");
        println!("Config file: {}", config_path.display());
    } else {
        println!(
            "{}: {} = {}",
            "Configuration updated".green().bold(),
            key,
            value
        );
        println!("{}: {}", "Config file".dimmed(), config_path.display());
    }

    Ok(0)
}

/// List all configuration settings
fn list(args: &ConfigArgs, changed: bool) -> Result<i32> {
    let config = Config::load_merged(args.project_root.as_deref())?;
    let defaults = Config::default();

    if args.json {
        list_json(&config)?;
    } else {
        list_text(&config, &defaults, changed, !args.no_color);
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
fn list_text(config: &Config, defaults: &Config, changed: bool, use_color: bool) {
    // Header
    if use_color {
        println!("{}", "Configuration Settings".cyan().bold());
        println!("{}", "=".repeat(50).dimmed());
    } else {
        println!("Configuration Settings");
        println!("{}", "=".repeat(50));
    }
    println!();

    // Output section
    print_section("[output]", use_color);
    print_setting(
        "  default_format",
        &config.output.default_format,
        changed && config.output.default_format == defaults.output.default_format,
        use_color,
    );
    print_setting(
        "  verbosity",
        &config.output.verbosity,
        changed && config.output.verbosity == defaults.output.verbosity,
        use_color,
    );
    print_setting(
        "  color",
        &config.output.color.to_string(),
        changed && config.output.color == defaults.output.color,
        use_color,
    );
    println!();

    // Linter section
    print_section("[linter]", use_color);
    print_setting(
        "  max_depth",
        &config.linter.max_depth.to_string(),
        changed && config.linter.max_depth == defaults.linter.max_depth,
        use_color,
    );
    print_setting(
        "  auto_fix",
        &config.linter.auto_fix.to_string(),
        changed && config.linter.auto_fix == defaults.linter.auto_fix,
        use_color,
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
        use_color,
    );
    println!();

    // Search section
    print_section("[search]", use_color);
    let threshold_unchanged =
        (config.search.fuzzy_threshold - defaults.search.fuzzy_threshold).abs() < f32::EPSILON;
    print_setting(
        "  fuzzy_threshold",
        &config.search.fuzzy_threshold.to_string(),
        changed && threshold_unchanged,
        use_color,
    );
    print_setting(
        "  limit",
        &config.search.limit.to_string(),
        changed && config.search.limit == defaults.search.limit,
        use_color,
    );
    println!();

    // Agent section
    print_section("[agent]", use_color);
    print_setting(
        "  token_budget",
        &config.agent.token_budget.to_string(),
        changed && config.agent.token_budget == defaults.agent.token_budget,
        use_color,
    );
    print_setting(
        "  default_format",
        &config.agent.default_format,
        changed && config.agent.default_format == defaults.agent.default_format,
        use_color,
    );
    println!();
}

/// Print a section header
fn print_section(name: &str, use_color: bool) {
    if use_color {
        println!("{}", name.yellow().bold());
    } else {
        println!("{name}");
    }
}

/// Print a single setting
fn print_setting(key: &str, value: &str, skip_if_unchanged: bool, use_color: bool) {
    // Skip unchanged settings if filtering for changed only
    if skip_if_unchanged {
        return;
    }

    if use_color {
        println!("{} = \"{}\"", key.green(), value);
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
}
