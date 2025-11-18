//! Configuration model for Lash projects

use crate::error::{codes, LashError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
}

impl Default for LashConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("."),
            index_file: "lash.index.md".to_string(),
            max_depth: 3,
            indent_spaces: 2,
            db_path: PathBuf::from(".lash/lash.db"),
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
    /// Returns `E_CFG_ROOT_NOT_FOUND` if no project root is found
    pub fn find_project_root(start_dir: &Path) -> Result<PathBuf> {
        let mut current = start_dir.canonicalize().map_err(|e| LashError::IoError {
            code: codes::E_IO_READ_ERROR,
            message: format!("Failed to canonicalize path: {e}"),
            path: Some(start_dir.to_path_buf()),
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
                    return Err(LashError::ConfigError {
                        code: codes::E_CFG_ROOT_NOT_FOUND,
                        message: format!(
                            "No Lash project found (searched from {})",
                            start_dir.display()
                        ),
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
            let content =
                std::fs::read_to_string(&config_path).map_err(|e| LashError::IoError {
                    code: codes::E_IO_READ_ERROR,
                    message: format!("Failed to read config file: {e}"),
                    path: Some(config_path.clone()),
                })?;

            let file_config: ConfigFile =
                toml::from_str(&content).map_err(|e| LashError::ConfigError {
                    code: codes::E_CFG_PARSE_ERROR,
                    message: format!("Failed to parse config file: {e}"),
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
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Validate max_depth
        if !(2..=5).contains(&self.max_depth) {
            return Err(LashError::ConfigError {
                code: codes::E_CFG_INVALID_VALUE,
                message: format!("max_depth must be between 2 and 5, got {}", self.max_depth),
            });
        }

        // Validate indent_spaces
        if self.indent_spaces != 2 && self.indent_spaces != 4 {
            return Err(LashError::ConfigError {
                code: codes::E_CFG_INVALID_VALUE,
                message: format!("indent_spaces must be 2 or 4, got {}", self.indent_spaces),
            });
        }

        // Validate root_path exists
        if !self.root_path.exists() {
            return Err(LashError::IoError {
                code: codes::E_IO_FILE_NOT_FOUND,
                message: format!("Root path does not exist: {}", self.root_path.display()),
                path: Some(self.root_path.clone()),
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

/// Configuration file structure (for TOML parsing)
#[derive(Debug, Deserialize)]
struct ConfigFile {
    index_file: Option<String>,
    max_depth: Option<u8>,
    indent_spaces: Option<u8>,
    db_path: Option<PathBuf>,
}

/// Builder for `LashConfig`
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    root_path: Option<PathBuf>,
    index_file: Option<String>,
    max_depth: Option<u8>,
    indent_spaces: Option<u8>,
    db_path: Option<PathBuf>,
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
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = LashConfig::default();
        assert_eq!(config.index_file, "lash.index.md");
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.indent_spaces, 2);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .root("/tmp")
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
        let result = ConfigBuilder::new().root("/tmp").max_depth(10).build();
        assert!(result.is_err());
        if let Err(LashError::ConfigError { code, .. }) = result {
            assert_eq!(code, codes::E_CFG_INVALID_VALUE);
        } else {
            panic!("Expected ConfigError");
        }
    }

    #[test]
    fn test_invalid_indent_spaces() {
        let result = ConfigBuilder::new().root("/tmp").indent_spaces(3).build();
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

        if let Err(LashError::ConfigError { code, .. }) = result {
            assert_eq!(code, codes::E_CFG_ROOT_NOT_FOUND);
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
}
