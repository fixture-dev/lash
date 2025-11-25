//! Configuration model for Lash projects

use crate::error::{codes, LashError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io::Write};

/// Lash project configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LashConfig {
    /// Project root directory
    pub root_path: PathBuf,

    /// Root index filename (default: "lash.index.md")
    pub index_file: String,

    /// Maximum task nesting depth (default: 3)
    pub max_depth: u8,

    /// Indentation size in spaces (default: 2)
    pub indent_spaces: u8,

    /// Database location (default: .lash/lash.db)
    pub db_path: PathBuf,

    /// Custom annotation keys (in addition to built-in keys)
    #[serde(default)]
    pub custom_annotation_keys: Vec<String>,
}

impl Default for LashConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("."),
            index_file: "lash.index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/lash.db"),
            custom_annotation_keys: Vec::new(),
        }
    }
}

impl LashConfig {
    /// Find the project root by searching upward for lash.index.md or .lash/
    ///
    /// # Arguments
    ///
    /// * `start_dir` - Directory to start searching from
    ///
    /// # Returns
    ///
    /// The project root directory if found
    ///
    /// # Errors
    ///
    /// Returns `E_CONFIG_ROOT_NOT_FOUND` if no project root is found
    pub fn find_project_root(start_dir: &Path) -> Result<PathBuf> {
        let mut current = start_dir.canonicalize().map_err(|e| LashError::IO {
            code: codes::E_IO_READ_ERROR,
            message: format!("Failed to canonicalize path: {e}"),
            path: Some(start_dir.to_path_buf()),
            io_error: Some(e.to_string()),
        })?;

        // Search upward until we find a marker
        loop {
            // Check for lash.index.md
            if current.join("lash.index.md").exists() {
                return Ok(current);
            }

            // Check for index.lash.md
            if current.join("index.lash.md").exists() {
                return Ok(current);
            }

            // Check for .lash directory
            if current.join(".lash").is_dir() {
                return Ok(current);
            }

            // Move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => {
                    return Err(LashError::Config {
                        code: codes::E_CONFIG_ROOT_NOT_FOUND,
                        message: format!(
                            "No Lash project found (searched from {})",
                            start_dir.display()
                        ),
                        path: Some(start_dir.to_path_buf()),
                        help: Some("run `lash init` to create a new lash project".to_string()),
                    });
                }
            }
        }
    }

    /// Load configuration from a directory
    ///
    /// Searches for .lash/config.toml and merges with defaults
    ///
    /// # Arguments
    ///
    /// * `root_path` - Project root directory
    ///
    /// # Errors
    ///
    /// Returns error if config file is invalid or cannot be read
    pub fn from_root(root_path: &Path) -> Result<Self> {
        let config_path = root_path.join(".lash/config.toml");

        let mut config = Self {
            root_path: root_path.to_path_buf(),
            db_path: root_path.join(".lash/lash.db"),
            ..Default::default()
        };

        // Load config file if it exists
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| LashError::IO {
                code: codes::E_IO_READ_ERROR,
                message: format!("Failed to read config file: {e}"),
                path: Some(config_path.clone()),
                io_error: Some(e.to_string()),
            })?;

            let file_config: ConfigFile =
                toml::from_str(&content).map_err(|e| LashError::Config {
                    code: codes::E_CONFIG_PARSE_ERROR,
                    message: format!("Failed to parse config file: {e}"),
                    path: Some(config_path.clone()),
                    help: Some("check that the configuration file is valid TOML".to_string()),
                })?;

            // Merge file config with defaults
            if let Some(index_file) = file_config.index_file {
                config.index_file = index_file;
            }
            if let Some(max_depth) = file_config.max_depth {
                config.max_depth = max_depth;
            }
            if let Some(indent_spaces) = file_config.indent_spaces {
                config.indent_spaces = indent_spaces;
            }
            if let Some(db_path) = file_config.db_path {
                config.db_path = if db_path.is_absolute() {
                    db_path
                } else {
                    root_path.join(&db_path)
                };
            }
            if let Some(custom_keys) = file_config.custom_annotation_keys {
                config.custom_annotation_keys = custom_keys;
            }
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Validate max_depth
        if !(2..=5).contains(&self.max_depth) {
            return Err(LashError::Config {
                code: codes::E_CONFIG_INVALID_VALUE,
                message: format!("max_depth must be between 2 and 5, got {}", self.max_depth),
                path: None,
                help: Some("max_depth must be between 2 and 5".to_string()),
            });
        }

        // Validate indent_spaces
        if self.indent_spaces != 2 && self.indent_spaces != 4 {
            return Err(LashError::Config {
                code: codes::E_CONFIG_INVALID_VALUE,
                message: format!("indent_spaces must be 2 or 4, got {}", self.indent_spaces),
                path: None,
                help: Some("indent_spaces must be either 2 or 4".to_string()),
            });
        }

        // Validate root_path exists
        if !self.root_path.exists() {
            return Err(LashError::IO {
                code: codes::E_IO_FILE_NOT_FOUND,
                message: format!("Root path does not exist: {}", self.root_path.display()),
                path: Some(self.root_path.clone()),
                io_error: None,
            });
        }

        Ok(())
    }

    /// Get the full path to the index file
    #[must_use]
    pub fn index_path(&self) -> PathBuf {
        self.root_path.join(&self.index_file)
    }
}

/// User-level configuration
///
/// Stored at `~/.lash/config.toml` for user preferences that apply across all projects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserConfig {
    /// Selected color scheme name (default: `Base2Tone Desert`)
    #[serde(default = "default_color_scheme")]
    pub color_scheme: String,

    /// Tree view configuration
    #[serde(default)]
    pub tree_view: TreeViewConfig,
}

fn default_color_scheme() -> String {
    "Base2Tone Desert".to_string()
}

/// Tree view configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeViewConfig {
    /// Enable tree view by default (default: true)
    #[serde(default = "default_tree_enabled")]
    pub enabled: bool,

    /// Maximum depth to display (default: 5)
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Start with all nodes expanded (default: false)
    #[serde(default = "default_expanded")]
    pub default_expanded: bool,

    /// Force ASCII mode instead of Unicode (default: false)
    #[serde(default = "default_ascii")]
    pub ascii_mode: bool,
}

fn default_tree_enabled() -> bool {
    true
}

fn default_max_depth() -> usize {
    5
}

fn default_expanded() -> bool {
    false
}

fn default_ascii() -> bool {
    false
}

impl Default for TreeViewConfig {
    fn default() -> Self {
        Self {
            enabled: default_tree_enabled(),
            max_depth: default_max_depth(),
            default_expanded: default_expanded(),
            ascii_mode: default_ascii(),
        }
    }
}

impl TreeViewConfig {
    /// Validate tree view configuration values
    ///
    /// # Errors
    ///
    /// Returns error if `max_depth` is outside the valid range (1-10)
    pub fn validate(&self) -> Result<()> {
        if !(1..=10).contains(&self.max_depth) {
            return Err(LashError::Config {
                code: codes::E_CONFIG_INVALID_VALUE,
                message: format!(
                    "tree_view.max_depth must be between 1 and 10, got {}",
                    self.max_depth
                ),
                path: None,
                help: Some("tree_view.max_depth must be between 1 and 10".to_string()),
            });
        }
        Ok(())
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            color_scheme: default_color_scheme(),
            tree_view: TreeViewConfig::default(),
        }
    }
}

impl UserConfig {
    /// Get the user config directory path (~/.lash)
    ///
    /// # Errors
    ///
    /// Returns error if home directory cannot be determined
    pub fn user_config_dir() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|home| home.join(".lash"))
            .ok_or_else(|| LashError::Config {
                code: codes::E_CONFIG_INVALID_VALUE,
                message: "Could not determine home directory".to_string(),
                path: None,
                help: Some("Set HOME environment variable".to_string()),
            })
    }

    /// Get the user config file path (~/.lash/config.toml)
    ///
    /// # Errors
    ///
    /// Returns error if home directory cannot be determined
    pub fn user_config_path() -> Result<PathBuf> {
        Ok(Self::user_config_dir()?.join("config.toml"))
    }

    /// Load user configuration from ~/.lash/config.toml
    ///
    /// If the file doesn't exist, returns default configuration.
    ///
    /// # Errors
    ///
    /// Returns error if config file exists but is invalid
    pub fn load() -> Result<Self> {
        let config_path = Self::user_config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path).map_err(|e| LashError::IO {
            code: codes::E_IO_READ_ERROR,
            message: format!("Failed to read user config file: {e}"),
            path: Some(config_path.clone()),
            io_error: Some(e.to_string()),
        })?;

        let config: UserConfig = toml::from_str(&content).map_err(|e| LashError::Config {
            code: codes::E_CONFIG_PARSE_ERROR,
            message: format!("Failed to parse user config file: {e}"),
            path: Some(config_path),
            help: Some("check that the configuration file is valid TOML".to_string()),
        })?;

        // Validate the tree view configuration
        config.tree_view.validate()?;

        Ok(config)
    }

    /// Save user configuration to ~/.lash/config.toml
    ///
    /// Creates the directory if it doesn't exist. Uses atomic writes to prevent corruption.
    ///
    /// # Errors
    ///
    /// Returns error if config cannot be written
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::user_config_dir()?;
        let config_path = Self::user_config_path()?;

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| LashError::IO {
                code: codes::E_IO_WRITE_ERROR,
                message: format!("Failed to create user config directory: {e}"),
                path: Some(config_dir.clone()),
                io_error: Some(e.to_string()),
            })?;
        }

        // Serialize to TOML
        let content = toml::to_string_pretty(self).map_err(|e| LashError::Config {
            code: codes::E_CONFIG_PARSE_ERROR,
            message: format!("Failed to serialize user config: {e}"),
            path: Some(config_path.clone()),
            help: None,
        })?;

        // Write atomically using a temporary file
        let tmp_path = config_path.with_extension("toml.tmp");
        let mut file = fs::File::create(&tmp_path).map_err(|e| LashError::IO {
            code: codes::E_IO_WRITE_ERROR,
            message: format!("Failed to create temporary config file: {e}"),
            path: Some(tmp_path.clone()),
            io_error: Some(e.to_string()),
        })?;

        file.write_all(content.as_bytes())
            .map_err(|e| LashError::IO {
                code: codes::E_IO_WRITE_ERROR,
                message: format!("Failed to write config file: {e}"),
                path: Some(tmp_path.clone()),
                io_error: Some(e.to_string()),
            })?;

        file.sync_all().map_err(|e| LashError::IO {
            code: codes::E_IO_WRITE_ERROR,
            message: format!("Failed to sync config file: {e}"),
            path: Some(tmp_path.clone()),
            io_error: Some(e.to_string()),
        })?;

        // Atomic rename
        fs::rename(&tmp_path, &config_path).map_err(|e| LashError::IO {
            code: codes::E_IO_WRITE_ERROR,
            message: format!("Failed to save config file: {e}"),
            path: Some(config_path),
            io_error: Some(e.to_string()),
        })?;

        Ok(())
    }
}

/// Configuration file structure (for TOML parsing)
#[derive(Debug, Deserialize)]
struct ConfigFile {
    index_file: Option<String>,
    max_depth: Option<u8>,
    indent_spaces: Option<u8>,
    db_path: Option<PathBuf>,
    custom_annotation_keys: Option<Vec<String>>,
}

/// Builder for `LashConfig`
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    root_path: Option<PathBuf>,
    index_file: Option<String>,
    max_depth: Option<u8>,
    indent_spaces: Option<u8>,
    db_path: Option<PathBuf>,
    custom_annotation_keys: Option<Vec<String>>,
}

impl ConfigBuilder {
    /// Create a new config builder with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the root path
    #[must_use]
    pub fn root(mut self, path: impl Into<PathBuf>) -> Self {
        self.root_path = Some(path.into());
        self
    }

    /// Set the index file name
    #[must_use]
    pub fn index_file(mut self, name: impl Into<String>) -> Self {
        self.index_file = Some(name.into());
        self
    }

    /// Set the maximum depth
    #[must_use]
    pub fn max_depth(mut self, depth: u8) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set the indentation size
    #[must_use]
    pub fn indent_spaces(mut self, spaces: u8) -> Self {
        self.indent_spaces = Some(spaces);
        self
    }

    /// Set the database path
    #[must_use]
    pub fn db_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.db_path = Some(path.into());
        self
    }

    /// Set custom annotation keys
    #[must_use]
    pub fn custom_annotation_keys(mut self, keys: Vec<String>) -> Self {
        self.custom_annotation_keys = Some(keys);
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration is invalid
    pub fn build(self) -> Result<LashConfig> {
        let root_path = self.root_path.unwrap_or_else(|| PathBuf::from("."));

        let config = LashConfig {
            root_path: root_path.clone(),
            index_file: self
                .index_file
                .unwrap_or_else(|| "lash.index.md".to_string()),
            max_depth: self.max_depth.unwrap_or(3),
            indent_spaces: self.indent_spaces.unwrap_or(2),
            db_path: self
                .db_path
                .unwrap_or_else(|| root_path.join(".lash/lash.db")),
            custom_annotation_keys: self.custom_annotation_keys.unwrap_or_default(),
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = LashConfig::default();
        assert_eq!(config.index_file, "lash.index.md");
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.indent_spaces, 2);
    }

    #[test]
    fn test_config_builder() {
        let temp_dir = TempDir::new().unwrap();
        let config = ConfigBuilder::new()
            .root(temp_dir.path())
            .max_depth(4)
            .indent_spaces(4)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.max_depth, 4);
        assert_eq!(config.indent_spaces, 4);
    }

    #[test]
    fn test_invalid_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let result = ConfigBuilder::new()
            .root(temp_dir.path())
            .max_depth(10)
            .build();
        assert!(result.is_err());
        if let Err(LashError::Config { code, .. }) = result {
            assert_eq!(code, codes::E_CONFIG_INVALID_VALUE);
        } else {
            panic!("Expected ConfigError");
        }
    }

    #[test]
    fn test_invalid_indent_spaces() {
        let temp_dir = TempDir::new().unwrap();
        let result = ConfigBuilder::new()
            .root(temp_dir.path())
            .indent_spaces(3)
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_find_project_root() {
        // Create a temporary test directory structure
        let temp_dir = std::env::temp_dir().join("lash_test_find_root");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create a nested directory
        let nested = temp_dir.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        // Create index file at root
        fs::write(temp_dir.join("lash.index.md"), "# Index").unwrap();

        // Search from nested directory
        let found = LashConfig::find_project_root(&nested).unwrap();
        assert_eq!(
            found.canonicalize().unwrap(),
            temp_dir.canonicalize().unwrap()
        );

        // Cleanup
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_find_project_root_not_found() {
        let temp_dir = std::env::temp_dir().join("lash_test_no_root");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let result = LashConfig::find_project_root(&temp_dir);
        assert!(result.is_err());

        if let Err(LashError::Config { code, .. }) = result {
            assert_eq!(code, codes::E_CONFIG_ROOT_NOT_FOUND);
        } else {
            panic!("Expected ConfigError");
        }

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_index_path() {
        let temp_dir = std::env::temp_dir().join("lash_test_index_path");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let config = ConfigBuilder::new()
            .root(&temp_dir)
            .index_file("my-index.md")
            .build()
            .unwrap();

        assert_eq!(config.index_path(), temp_dir.join("my-index.md"));

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_user_config_default() {
        let config = UserConfig::default();
        assert_eq!(config.color_scheme, "Base2Tone Desert");
        assert!(config.tree_view.enabled);
        assert_eq!(config.tree_view.max_depth, 5);
        assert!(!config.tree_view.default_expanded);
        assert!(!config.tree_view.ascii_mode);
    }

    #[test]
    fn test_tree_view_config_default() {
        let config = TreeViewConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_depth, 5);
        assert!(!config.default_expanded);
        assert!(!config.ascii_mode);
    }

    #[test]
    fn test_tree_view_config_validation_valid() {
        let config = TreeViewConfig {
            enabled: true,
            max_depth: 1,
            default_expanded: false,
            ascii_mode: false,
        };
        assert!(config.validate().is_ok());

        let config = TreeViewConfig {
            enabled: true,
            max_depth: 10,
            default_expanded: false,
            ascii_mode: false,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tree_view_config_validation_invalid() {
        let config = TreeViewConfig {
            enabled: true,
            max_depth: 0,
            default_expanded: false,
            ascii_mode: false,
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(LashError::Config { code, .. }) = result {
            assert_eq!(code, codes::E_CONFIG_INVALID_VALUE);
        }

        let config = TreeViewConfig {
            enabled: true,
            max_depth: 11,
            default_expanded: false,
            ascii_mode: false,
        };
        let result = config.validate();
        assert!(result.is_err());
        if let Err(LashError::Config { code, .. }) = result {
            assert_eq!(code, codes::E_CONFIG_INVALID_VALUE);
        }
    }

    #[test]
    fn test_tree_view_config_serialization() {
        let config = TreeViewConfig {
            enabled: true,
            max_depth: 7,
            default_expanded: true,
            ascii_mode: true,
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: TreeViewConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, deserialized);
        assert!(deserialized.enabled);
        assert_eq!(deserialized.max_depth, 7);
        assert!(deserialized.default_expanded);
        assert!(deserialized.ascii_mode);
    }

    #[test]
    fn test_user_config_load_nonexistent() {
        // This test assumes ~/.lash/config.toml doesn't exist or is valid
        // If it exists, it should parse successfully
        let result = UserConfig::load();
        assert!(result.is_ok());
    }

    #[test]
    fn test_user_config_save_and_load() {
        // Create a custom config
        let config = UserConfig {
            color_scheme: "Test Theme".to_string(),
            tree_view: TreeViewConfig::default(),
        };

        // Save it
        let save_result = config.save();
        assert!(
            save_result.is_ok(),
            "Failed to save config: {:?}",
            save_result
        );

        // Load it back
        let loaded = UserConfig::load().unwrap();
        assert_eq!(loaded.color_scheme, "Test Theme");

        // Restore default
        UserConfig::default().save().unwrap();
    }
}
