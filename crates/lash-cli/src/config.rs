//! Configuration management
//!
//! This module implements project-level and user-level configuration file support,
//! allowing customization of Lash behavior through TOML configuration files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration struct
///
/// Holds all configuration settings for Lash, with defaults provided for all fields.
/// Configuration can be loaded from TOML files and merged with CLI arguments.
///
/// # Example
///
/// ```no_run
/// use lash_cli::config::Config;
/// use std::path::Path;
///
/// let config = Config::load_from_file(Path::new(".lash/config.toml"))
///     .unwrap_or_default();
/// println!("Output format: {:?}", config.output.default_format);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Output-related settings
    #[serde(default)]
    pub output: OutputConfig,

    /// Linter-related settings
    #[serde(default)]
    pub linter: LinterConfig,

    /// Search-related settings
    #[serde(default)]
    pub search: SearchConfig,

    /// Agent-related settings
    #[serde(default)]
    pub agent: AgentConfig,
}

/// Output formatting configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    /// Default output format (text, json, json-pretty)
    #[serde(default = "default_output_format")]
    pub default_format: String,

    /// Default verbosity level (quiet, normal, verbose, debug)
    #[serde(default = "default_verbosity")]
    pub verbosity: String,

    /// Enable colored output by default
    #[serde(default = "default_true")]
    pub color: bool,
}

/// Linter configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LinterConfig {
    /// Maximum task nesting depth
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Enable auto-fix by default
    #[serde(default = "default_false")]
    pub auto_fix: bool,

    /// Rules to enable/disable
    #[serde(default)]
    pub rules: Vec<String>,

    /// Maximum description length threshold (warning)
    ///
    /// If set, overrides the default warning threshold (1000 characters)
    /// for the `W_SEM_DESC_TOO_LONG` rule. The error threshold is
    /// automatically set to 2x this value.
    #[serde(default)]
    pub description_max_length: Option<usize>,
}

/// Search configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SearchConfig {
    /// Fuzzy matching threshold (0.0 = exact, 1.0 = very fuzzy)
    #[serde(default = "default_fuzzy_threshold")]
    pub fuzzy_threshold: f32,

    /// Default result limit
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

/// Agent integration configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    /// Default token budget for agent prompts
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,

    /// Default agent prompt format
    #[serde(default = "default_agent_format")]
    pub default_format: String,
}

// Default value functions
fn default_output_format() -> String {
    "text".to_string()
}

fn default_verbosity() -> String {
    "normal".to_string()
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_max_depth() -> usize {
    4
}

fn default_fuzzy_threshold() -> f32 {
    0.3
}

fn default_search_limit() -> usize {
    20
}

fn default_token_budget() -> usize {
    4000
}

fn default_agent_format() -> String {
    "plain".to_string()
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            default_format: default_output_format(),
            verbosity: default_verbosity(),
            color: default_true(),
        }
    }
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            auto_fix: default_false(),
            rules: Vec::new(),
            description_max_length: None,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            fuzzy_threshold: default_fuzzy_threshold(),
            limit: default_search_limit(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            token_budget: default_token_budget(),
            default_format: default_agent_format(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The parsed configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be read
    /// - TOML parsing fails
    /// - Configuration validation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::config::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::load_from_file(Path::new(".lash/config.toml"))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    /// Load configuration with merging strategy
    ///
    /// Loads and merges configuration from multiple sources with the following priority:
    /// 1. Project config (`.lash/config.toml` in project root) - highest priority
    /// 2. User config (`~/.config/lash/config.toml`)
    /// 3. Default values - lowest priority
    ///
    /// # Arguments
    ///
    /// * `project_root` - Optional path to project root (for project config)
    ///
    /// # Returns
    ///
    /// The merged configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use lash_cli::config::Config;
    /// use std::path::Path;
    ///
    /// let config = Config::load_merged(Some(Path::new("/path/to/project")))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn load_merged(project_root: Option<&Path>) -> Result<Self> {
        let mut config = Config::default();

        // Load user config if it exists
        if let Some(user_config_path) = Self::user_config_path() {
            if user_config_path.exists() {
                let user_config = Self::load_from_file(&user_config_path)?;
                config = config.merge_with(user_config);
            }
        }

        // Load project config if it exists (takes priority)
        if let Some(root) = project_root {
            let project_config_path = root.join(".lash").join("config.toml");
            if project_config_path.exists() {
                let project_config = Self::load_from_file(&project_config_path)?;
                config = config.merge_with(project_config);
            }
        }

        Ok(config)
    }

    /// Get the user configuration file path
    ///
    /// Returns `~/.config/lash/config.toml` on Unix-like systems
    ///
    /// # Example
    ///
    /// ```
    /// use lash_cli::config::Config;
    ///
    /// if let Some(path) = Config::user_config_path() {
    ///     println!("User config: {}", path.display());
    /// }
    /// ```
    #[must_use]
    pub fn user_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("lash").join("config.toml"))
    }

    /// Merge this configuration with another, with the other taking priority
    ///
    /// # Arguments
    ///
    /// * `other` - Configuration to merge (takes priority over self)
    ///
    /// # Returns
    ///
    /// A new configuration with merged values
    #[must_use]
    pub fn merge_with(self, other: Config) -> Config {
        Config {
            output: OutputConfig {
                default_format: if other.output.default_format == default_output_format() {
                    self.output.default_format
                } else {
                    other.output.default_format
                },
                verbosity: if other.output.verbosity == default_verbosity() {
                    self.output.verbosity
                } else {
                    other.output.verbosity
                },
                color: other.output.color,
            },
            linter: LinterConfig {
                max_depth: if other.linter.max_depth == default_max_depth() {
                    self.linter.max_depth
                } else {
                    other.linter.max_depth
                },
                auto_fix: other.linter.auto_fix,
                rules: if other.linter.rules.is_empty() {
                    self.linter.rules
                } else {
                    other.linter.rules
                },
                description_max_length: other
                    .linter
                    .description_max_length
                    .or(self.linter.description_max_length),
            },
            search: SearchConfig {
                fuzzy_threshold: if (other.search.fuzzy_threshold - default_fuzzy_threshold()).abs()
                    > f32::EPSILON
                {
                    other.search.fuzzy_threshold
                } else {
                    self.search.fuzzy_threshold
                },
                limit: if other.search.limit == default_search_limit() {
                    self.search.limit
                } else {
                    other.search.limit
                },
            },
            agent: AgentConfig {
                token_budget: if other.agent.token_budget == default_token_budget() {
                    self.agent.token_budget
                } else {
                    other.agent.token_budget
                },
                default_format: if other.agent.default_format == default_agent_format() {
                    self.agent.default_format
                } else {
                    other.agent.default_format
                },
            },
        }
    }

    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration value is invalid
    pub fn validate(&self) -> Result<()> {
        // Validate output format
        match self.output.default_format.as_str() {
            "text" | "json" | "json-pretty" => {}
            _ => anyhow::bail!(
                "Invalid output format: '{}'. Must be one of: text, json, json-pretty",
                self.output.default_format
            ),
        }

        // Validate verbosity
        match self.output.verbosity.as_str() {
            "quiet" | "normal" | "verbose" | "debug" => {}
            _ => anyhow::bail!(
                "Invalid verbosity: '{}'. Must be one of: quiet, normal, verbose, debug",
                self.output.verbosity
            ),
        }

        // Validate linter max depth
        if self.linter.max_depth == 0 || self.linter.max_depth > 10 {
            anyhow::bail!(
                "Invalid linter max_depth: {}. Must be between 1 and 10",
                self.linter.max_depth
            );
        }

        // Validate search fuzzy threshold
        if !(0.0..=1.0).contains(&self.search.fuzzy_threshold) {
            anyhow::bail!(
                "Invalid search fuzzy_threshold: {}. Must be between 0.0 and 1.0",
                self.search.fuzzy_threshold
            );
        }

        // Validate search limit
        if self.search.limit == 0 || self.search.limit > 1000 {
            anyhow::bail!(
                "Invalid search limit: {}. Must be between 1 and 1000",
                self.search.limit
            );
        }

        // Validate agent token budget
        if self.agent.token_budget < 100 || self.agent.token_budget > 100_000 {
            anyhow::bail!(
                "Invalid agent token_budget: {}. Must be between 100 and 100000",
                self.agent.token_budget
            );
        }

        // Validate agent format
        match self.agent.default_format.as_str() {
            "plain" | "json" | "agents-md" => {}
            _ => anyhow::bail!(
                "Invalid agent format: '{}'. Must be one of: plain, json, agents-md. \
                 (The 'claude-skill' format was removed; use `lash skill install --target claude` instead.)",
                self.agent.default_format
            ),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.output.default_format, "text");
        assert_eq!(config.output.verbosity, "normal");
        assert!(config.output.color);
        assert_eq!(config.linter.max_depth, 4);
        assert!(!config.linter.auto_fix);
        assert!((config.search.fuzzy_threshold - 0.3).abs() < f32::EPSILON);
        assert_eq!(config.search.limit, 20);
        assert_eq!(config.agent.token_budget, 4000);
    }

    #[test]
    fn test_load_from_file_valid() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let toml = r#"
[output]
default_format = "json"
verbosity = "verbose"
color = false

[linter]
max_depth = 3
auto_fix = true

[search]
fuzzy_threshold = 0.5
limit = 50

[agent]
token_budget = 8000
default_format = "json"
"#;
        std::fs::write(&config_path, toml).unwrap();

        let config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.output.default_format, "json");
        assert_eq!(config.output.verbosity, "verbose");
        assert!(!config.output.color);
        assert_eq!(config.linter.max_depth, 3);
        assert!(config.linter.auto_fix);
        assert!((config.search.fuzzy_threshold - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.search.limit, 50);
        assert_eq!(config.agent.token_budget, 8000);
    }

    #[test]
    fn test_load_from_file_partial() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let toml = r#"
[output]
default_format = "json"

[search]
limit = 100
"#;
        std::fs::write(&config_path, toml).unwrap();

        let config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.output.default_format, "json");
        assert_eq!(config.output.verbosity, "normal"); // default
        assert_eq!(config.search.limit, 100);
        assert!((config.search.fuzzy_threshold - 0.3).abs() < f32::EPSILON); // default
    }

    #[test]
    fn test_load_from_file_invalid_format() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let toml = r#"
[output]
default_format = "invalid"
"#;
        std::fs::write(&config_path, toml).unwrap();

        let result = Config::load_from_file(&config_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid output format"));
    }

    #[test]
    fn test_validate_invalid_verbosity() {
        let mut config = Config::default();
        config.output.verbosity = "invalid".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid verbosity"));
    }

    #[test]
    fn test_validate_invalid_max_depth() {
        let mut config = Config::default();
        config.linter.max_depth = 0;
        let result = config.validate();
        assert!(result.is_err());

        config.linter.max_depth = 11;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_fuzzy_threshold() {
        let mut config = Config::default();
        config.search.fuzzy_threshold = 1.5;
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_with() {
        let mut base = Config::default();
        base.output.default_format = "text".to_string();
        base.linter.max_depth = 4;

        let mut override_config = Config::default();
        override_config.output.default_format = "json".to_string();
        override_config.search.limit = 100;

        let merged = base.merge_with(override_config);
        assert_eq!(merged.output.default_format, "json");
        assert_eq!(merged.linter.max_depth, 4); // unchanged
        assert_eq!(merged.search.limit, 100);
    }

    #[test]
    fn test_load_merged_no_files() {
        let config = Config::load_merged(None).unwrap();
        assert_eq!(config.output.default_format, "text");
    }

    #[test]
    fn test_load_merged_with_project_config() {
        let temp = TempDir::new().unwrap();
        let lash_dir = temp.path().join(".lash");
        std::fs::create_dir(&lash_dir).unwrap();

        let config_path = lash_dir.join("config.toml");
        let toml = r#"
[output]
default_format = "json"
"#;
        std::fs::write(&config_path, toml).unwrap();

        let config = Config::load_merged(Some(temp.path())).unwrap();
        assert_eq!(config.output.default_format, "json");
    }

    #[test]
    fn test_user_config_path() {
        let path = Config::user_config_path();
        assert!(path.is_some());
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("lash"));
            assert!(p.to_string_lossy().contains("config.toml"));
        }
    }

    #[test]
    fn test_unknown_field_rejected() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");

        let toml = r#"
[output]
default_format = "json"
unknown_field = "value"
"#;
        std::fs::write(&config_path, toml).unwrap();

        let result = Config::load_from_file(&config_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        // The error should indicate that the field is unknown
        // TOML deserialization might report this as "unknown field" or just parsing error
        eprintln!("Error message: {err_msg}");
        assert!(
            err_msg.to_lowercase().contains("unknown")
                || err_msg.contains("field")
                || err_msg.contains("parse")
        );
    }
}
