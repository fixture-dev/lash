//! Logging and diagnostics configuration
//!
//! This module provides structured logging using the `tracing` ecosystem.
//! It supports multiple output formats (text, JSON), configurable log levels,
//! and optional file logging.

use crate::formatter::Verbosity;
use anyhow::{Context, Result};
use std::io;
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Configuration for logging initialization
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Verbosity level from CLI flags
    pub verbosity: Verbosity,
    /// Whether to output logs as JSON
    pub json_output: bool,
    /// Whether to suppress colored output
    pub no_color: bool,
    /// Optional file path for log output
    pub log_file: Option<PathBuf>,
}

impl LogConfig {
    /// Create a new log configuration
    ///
    /// # Arguments
    ///
    /// * `verbosity` - Verbosity level from CLI flags
    /// * `json_output` - Whether to use JSON output format
    /// * `no_color` - Whether to disable colored output
    ///
    /// # Examples
    ///
    /// ```
    /// use lash::logging::LogConfig;
    /// use lash::formatter::Verbosity;
    ///
    /// let config = LogConfig::new(Verbosity::Normal, false, false);
    /// ```
    #[must_use]
    pub fn new(verbosity: Verbosity, json_output: bool, no_color: bool) -> Self {
        Self {
            verbosity,
            json_output,
            no_color,
            log_file: None,
        }
    }

    /// Set the log file path
    ///
    /// # Examples
    ///
    /// ```
    /// use lash::logging::LogConfig;
    /// use lash::formatter::Verbosity;
    /// use std::path::PathBuf;
    ///
    /// let config = LogConfig::new(Verbosity::Normal, false, false)
    ///     .with_log_file(PathBuf::from("/tmp/lash.log"));
    /// ```
    #[must_use]
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_file = Some(path);
        self
    }

    /// Determine the tracing level based on verbosity and environment variables
    ///
    /// Priority order:
    /// 1. `LASH_LOG` environment variable
    /// 2. `RUST_LOG` environment variable
    /// 3. Verbosity flag from CLI
    ///
    /// # Examples
    ///
    /// ```
    /// use lash::logging::LogConfig;
    /// use lash::formatter::Verbosity;
    ///
    /// let config = LogConfig::new(Verbosity::Normal, false, false);
    /// let level = config.determine_level();
    /// ```
    #[must_use]
    pub fn determine_level(&self) -> Level {
        // Check `LASH_LOG` first
        if let Ok(level_str) = std::env::var("LASH_LOG") {
            if let Ok(level) = parse_log_level(&level_str) {
                return level;
            }
        }

        // Check `RUST_LOG` as fallback
        if let Ok(level_str) = std::env::var("RUST_LOG") {
            if let Ok(level) = parse_log_level(&level_str) {
                return level;
            }
        }

        // Use verbosity flag
        verbosity_to_level(self.verbosity)
    }
}

/// Initialize the logging system
///
/// This function should be called once at the start of the application.
/// It configures the tracing subscriber based on the provided configuration.
///
/// # Arguments
///
/// * `config` - Logging configuration
///
/// # Errors
///
/// Returns an error if the logging system cannot be initialized.
///
/// # Examples
///
/// ```no_run
/// use lash::logging::{LogConfig, init_logging};
/// use lash::formatter::Verbosity;
///
/// let config = LogConfig::new(Verbosity::Normal, false, false);
/// init_logging(&config)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn init_logging(config: &LogConfig) -> Result<()> {
    let level = config.determine_level();

    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "lash={level},lash={level},lash_core={level},lash_db={level}"
        ))
    });

    if config.json_output {
        // JSON output mode - emit logs as JSON events to stderr
        let json_layer = fmt::layer()
            .json()
            .with_writer(io::stderr)
            .with_current_span(true)
            .with_span_list(true)
            .with_target(true)
            .with_filter(env_filter);

        tracing_subscriber::registry().with(json_layer).init();
    } else {
        // Terminal output mode - compact, human-readable format
        let fmt_layer = fmt::layer()
            .compact()
            .with_writer(io::stderr)
            .with_ansi(!config.no_color && supports_color())
            .with_target(false)
            .with_span_events(FmtSpan::NONE)
            .with_filter(env_filter);

        tracing_subscriber::registry().with(fmt_layer).init();
    }

    tracing::debug!(
        verbosity = ?config.verbosity,
        level = ?level,
        json = config.json_output,
        "Logging initialized"
    );

    Ok(())
}

/// Initialize the logging system with optional file output
///
/// This variant creates a non-blocking file appender for log files.
///
/// # Arguments
///
/// * `config` - Logging configuration with log file path
///
/// # Errors
///
/// Returns an error if the logging system or file appender cannot be initialized.
///
/// # Examples
///
/// ```no_run
/// use lash::logging::{LogConfig, init_logging_with_file};
/// use lash::formatter::Verbosity;
/// use std::path::PathBuf;
///
/// let config = LogConfig::new(Verbosity::Normal, false, false)
///     .with_log_file(PathBuf::from("/tmp/lash.log"));
/// init_logging_with_file(&config)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn init_logging_with_file(config: &LogConfig) -> Result<()> {
    let level = config.determine_level();

    // Create environment filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "lash={level},lash={level},lash_core={level},lash_db={level}"
        ))
    });

    if let Some(log_file) = &config.log_file {
        // Set up file appender
        let file_parent = log_file
            .parent()
            .context("Log file path has no parent directory")?;
        let file_name = log_file
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid log file name")?;

        let file_appender = tracing_appender::rolling::never(file_parent, file_name);
        let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

        if config.json_output {
            // JSON output to stderr and file
            let stderr_layer = fmt::layer()
                .json()
                .with_writer(io::stderr)
                .with_current_span(true)
                .with_span_list(true)
                .with_target(true);

            let file_layer = fmt::layer()
                .json()
                .with_writer(non_blocking_file)
                .with_current_span(true)
                .with_span_list(true)
                .with_target(true);

            tracing_subscriber::registry()
                .with(stderr_layer.with_filter(env_filter.clone()))
                .with(file_layer.with_filter(env_filter))
                .init();
        } else {
            // Terminal output to stderr, JSON to file
            let stderr_layer = fmt::layer()
                .compact()
                .with_writer(io::stderr)
                .with_ansi(!config.no_color && supports_color())
                .with_target(false)
                .with_span_events(FmtSpan::NONE);

            let file_layer = fmt::layer()
                .json()
                .with_writer(non_blocking_file)
                .with_current_span(true)
                .with_span_list(true)
                .with_target(true);

            tracing_subscriber::registry()
                .with(stderr_layer.with_filter(env_filter.clone()))
                .with(file_layer.with_filter(env_filter))
                .init();
        }

        tracing::debug!(
            log_file = ?log_file,
            "File logging enabled"
        );
    } else {
        // Fall back to regular initialization
        init_logging(config)?;
    }

    Ok(())
}

/// Set up a panic hook that logs panics with backtrace
///
/// This function installs a panic hook that captures panic information
/// and logs it using the tracing infrastructure. It also suggests filing
/// a bug report with the logs.
///
/// # Examples
///
/// ```
/// use lash::logging::install_panic_hook;
///
/// install_panic_hook();
/// ```
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        // Get panic location
        let location = panic_info.location().map_or_else(
            || String::from("<unknown>"),
            |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
        );

        // Get panic message
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            String::from("<no message>")
        };

        // Log the panic
        tracing::error!(
            location = %location,
            message = %message,
            backtrace = ?std::backtrace::Backtrace::force_capture(),
            "Panic occurred"
        );

        // Print user-friendly error message
        let separator = "=".repeat(80);
        eprintln!("\n{separator}");
        eprintln!("Lash encountered an unexpected error and crashed.");
        eprintln!();
        eprintln!("Location: {location}");
        eprintln!("Message:  {message}");
        eprintln!();
        eprintln!("This is likely a bug in Lash. Please consider filing a bug report at:");
        eprintln!("  https://github.com/fixture-dev/lash/issues");
        eprintln!();
        eprintln!("Please include:");
        eprintln!("  - The command you ran");
        eprintln!("  - Your OS and Lash version (lash --version)");
        eprintln!("  - Any relevant error messages or logs");
        eprintln!("{separator}");

        // Call the default hook to print the standard panic message
        default_hook(panic_info);
    }));
}

/// Get version and platform information for diagnostic messages
///
/// # Examples
///
/// ```
/// use lash::logging::get_diagnostic_info;
///
/// let info = get_diagnostic_info();
/// println!("Lash version: {}", info.version);
/// ```
#[must_use]
pub fn get_diagnostic_info() -> DiagnosticInfo {
    DiagnosticInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// Diagnostic information about the current runtime environment
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    /// Lash version
    pub version: String,
    /// Operating system
    pub platform: String,
    /// CPU architecture
    pub arch: String,
}

/// Map verbosity level to tracing level
///
/// # Examples
///
/// ```
/// use lash::logging::verbosity_to_level;
/// use lash::formatter::Verbosity;
/// use tracing::Level;
///
/// assert_eq!(verbosity_to_level(Verbosity::Quiet), Level::ERROR);
/// assert_eq!(verbosity_to_level(Verbosity::Normal), Level::WARN);
/// assert_eq!(verbosity_to_level(Verbosity::Verbose), Level::INFO);
/// assert_eq!(verbosity_to_level(Verbosity::Debug), Level::DEBUG);
/// assert_eq!(verbosity_to_level(Verbosity::Trace), Level::TRACE);
/// ```
#[must_use]
pub fn verbosity_to_level(verbosity: Verbosity) -> Level {
    match verbosity {
        Verbosity::Quiet => Level::ERROR,
        Verbosity::Normal => Level::WARN,
        Verbosity::Verbose => Level::INFO,
        Verbosity::Debug => Level::DEBUG,
        Verbosity::Trace => Level::TRACE,
    }
}

/// Parse a log level string
///
/// # Examples
///
/// ```
/// use lash::logging::parse_log_level;
/// use tracing::Level;
///
/// assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);
/// assert_eq!(parse_log_level("warn").unwrap(), Level::WARN);
/// assert_eq!(parse_log_level("info").unwrap(), Level::INFO);
/// assert_eq!(parse_log_level("debug").unwrap(), Level::DEBUG);
/// assert_eq!(parse_log_level("trace").unwrap(), Level::TRACE);
/// assert!(parse_log_level("invalid").is_err());
/// ```
pub fn parse_log_level(s: &str) -> Result<Level, String> {
    match s.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" | "warning" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(format!("Invalid log level: {s}")),
    }
}

/// Check if the terminal supports color output
///
/// Respects the `NO_COLOR` environment variable and checks if stderr is a TTY.
fn supports_color() -> bool {
    // NO_COLOR environment variable takes precedence
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // Check if stderr is a TTY (logs go to stderr)
    atty::is(atty::Stream::Stderr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::panic;

    #[test]
    fn test_verbosity_to_level() {
        assert_eq!(verbosity_to_level(Verbosity::Quiet), Level::ERROR);
        assert_eq!(verbosity_to_level(Verbosity::Normal), Level::WARN);
        assert_eq!(verbosity_to_level(Verbosity::Verbose), Level::INFO);
        assert_eq!(verbosity_to_level(Verbosity::Debug), Level::DEBUG);
        assert_eq!(verbosity_to_level(Verbosity::Trace), Level::TRACE);
    }

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("ERROR").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("warn").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("warning").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("WARNING").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("info").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("INFO").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("debug").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("DEBUG").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("trace").unwrap(), Level::TRACE);
        assert_eq!(parse_log_level("TRACE").unwrap(), Level::TRACE);
        assert!(parse_log_level("invalid").is_err());
        assert!(parse_log_level("").is_err());
        assert!(parse_log_level("CRITICAL").is_err());
    }

    #[test]
    fn test_log_config_new() {
        let config = LogConfig::new(Verbosity::Normal, false, false);
        assert_eq!(config.verbosity, Verbosity::Normal);
        assert!(!config.json_output);
        assert!(!config.no_color);
        assert!(config.log_file.is_none());
    }

    #[test]
    fn test_log_config_new_with_json() {
        let config = LogConfig::new(Verbosity::Verbose, true, false);
        assert_eq!(config.verbosity, Verbosity::Verbose);
        assert!(config.json_output);
        assert!(!config.no_color);
    }

    #[test]
    fn test_log_config_new_with_no_color() {
        let config = LogConfig::new(Verbosity::Normal, false, true);
        assert_eq!(config.verbosity, Verbosity::Normal);
        assert!(!config.json_output);
        assert!(config.no_color);
    }

    #[test]
    fn test_log_config_with_log_file() {
        let config = LogConfig::new(Verbosity::Normal, false, false)
            .with_log_file(PathBuf::from("/tmp/test.log"));
        assert_eq!(config.log_file, Some(PathBuf::from("/tmp/test.log")));
    }

    #[test]
    fn test_log_config_builder_pattern() {
        let config = LogConfig::new(Verbosity::Debug, true, true)
            .with_log_file(PathBuf::from("/var/log/lash.log"));
        assert_eq!(config.verbosity, Verbosity::Debug);
        assert!(config.json_output);
        assert!(config.no_color);
        assert_eq!(config.log_file, Some(PathBuf::from("/var/log/lash.log")));
    }

    #[test]
    #[serial]
    fn test_log_config_determine_level_from_verbosity() {
        // Clear env vars to ensure we test verbosity-based level
        std::env::remove_var("LASH_LOG");
        std::env::remove_var("RUST_LOG");

        let config = LogConfig::new(Verbosity::Debug, false, false);
        assert_eq!(config.determine_level(), Level::DEBUG);

        let config = LogConfig::new(Verbosity::Quiet, false, false);
        assert_eq!(config.determine_level(), Level::ERROR);

        let config = LogConfig::new(Verbosity::Trace, false, false);
        assert_eq!(config.determine_level(), Level::TRACE);
    }

    #[test]
    #[serial]
    fn test_log_config_determine_level_from_lash_log_env() {
        // LASH_LOG takes priority
        std::env::set_var("LASH_LOG", "info");
        std::env::remove_var("RUST_LOG");

        let config = LogConfig::new(Verbosity::Debug, false, false);
        assert_eq!(config.determine_level(), Level::INFO);

        std::env::remove_var("LASH_LOG");
    }

    #[test]
    #[serial]
    fn test_log_config_determine_level_from_rust_log_env() {
        // RUST_LOG is fallback
        std::env::remove_var("LASH_LOG");
        std::env::set_var("RUST_LOG", "trace");

        let config = LogConfig::new(Verbosity::Normal, false, false);
        assert_eq!(config.determine_level(), Level::TRACE);

        std::env::remove_var("RUST_LOG");
    }

    #[test]
    #[serial]
    fn test_log_config_determine_level_priority() {
        // LASH_LOG should override RUST_LOG
        std::env::set_var("LASH_LOG", "error");
        std::env::set_var("RUST_LOG", "debug");

        let config = LogConfig::new(Verbosity::Trace, false, false);
        assert_eq!(config.determine_level(), Level::ERROR);

        std::env::remove_var("LASH_LOG");
        std::env::remove_var("RUST_LOG");
    }

    #[test]
    #[serial]
    fn test_log_config_determine_level_invalid_env() {
        // Invalid env var should fall back to verbosity
        std::env::set_var("LASH_LOG", "invalid_level");
        std::env::remove_var("RUST_LOG");

        let config = LogConfig::new(Verbosity::Verbose, false, false);
        assert_eq!(config.determine_level(), Level::INFO);

        std::env::remove_var("LASH_LOG");
    }

    #[test]
    fn test_get_diagnostic_info() {
        let info = get_diagnostic_info();
        assert!(!info.version.is_empty());
        assert!(!info.platform.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn test_get_diagnostic_info_contains_version() {
        let info = get_diagnostic_info();
        // Should be a valid semver-like version
        assert!(info.version.contains('.'));
    }

    #[test]
    fn test_get_diagnostic_info_platform_values() {
        let info = get_diagnostic_info();
        // Should be one of the known platforms
        assert!(
            info.platform == "linux"
                || info.platform == "macos"
                || info.platform == "windows"
                || info.platform == "freebsd"
                || info.platform == "openbsd"
                || info.platform == "netbsd"
                || info.platform == "dragonfly"
                || info.platform == "android"
                || info.platform == "ios"
        );
    }

    #[test]
    fn test_install_panic_hook() {
        // Just ensure it doesn't crash
        install_panic_hook();
    }

    #[test]
    fn test_install_panic_hook_idempotent() {
        // Should be safe to call multiple times
        install_panic_hook();
        install_panic_hook();
    }

    #[test]
    #[should_panic(expected = "test panic")]
    fn test_panic_hook_with_string_message() {
        install_panic_hook();
        panic!("test panic");
    }

    #[test]
    #[serial]
    fn test_supports_color_with_no_color_env() {
        // NO_COLOR environment variable should disable color
        std::env::set_var("NO_COLOR", "1");
        assert!(!supports_color());
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    #[serial]
    fn test_supports_color_without_no_color_env() {
        std::env::remove_var("NO_COLOR");
        // Result depends on whether stderr is a TTY, which varies by test environment
        // Just ensure it doesn't crash
        let _result = supports_color();
    }

    // Note: We cannot easily test init_logging() and init_logging_with_file()
    // because they call tracing_subscriber::registry().init(), which can only
    // be called once per process. They are effectively integration tests that
    // require separate test processes.
    //
    // However, we can test the configuration logic and ensure the functions
    // don't panic with various configurations.

    #[test]
    fn test_log_config_all_verbosity_levels() {
        // Ensure all verbosity levels produce valid configurations
        for verbosity in [
            Verbosity::Quiet,
            Verbosity::Normal,
            Verbosity::Verbose,
            Verbosity::Debug,
            Verbosity::Trace,
        ] {
            let config = LogConfig::new(verbosity, false, false);
            let level = verbosity_to_level(verbosity);
            let determined_level = config.determine_level();
            assert!(matches!(
                level,
                Level::ERROR | Level::WARN | Level::INFO | Level::DEBUG | Level::TRACE
            ));
            assert!(matches!(
                determined_level,
                Level::ERROR | Level::WARN | Level::INFO | Level::DEBUG | Level::TRACE
            ));
        }
    }

    #[test]
    fn test_log_config_all_output_modes() {
        // Test various configuration combinations
        let configs = [
            LogConfig::new(Verbosity::Normal, false, false),
            LogConfig::new(Verbosity::Normal, true, false),
            LogConfig::new(Verbosity::Normal, false, true),
            LogConfig::new(Verbosity::Normal, true, true),
        ];

        for config in &configs {
            // Just ensure the configuration is valid
            let _level = config.determine_level();
        }
    }

    #[test]
    fn test_diagnostic_info_debug_format() {
        let info = get_diagnostic_info();
        let debug_str = format!("{info:?}");
        assert!(debug_str.contains("DiagnosticInfo"));
        assert!(debug_str.contains("version"));
        assert!(debug_str.contains("platform"));
        assert!(debug_str.contains("arch"));
    }

    #[test]
    fn test_diagnostic_info_clone() {
        let info1 = get_diagnostic_info();
        let info2 = info1.clone();
        assert_eq!(info1.version, info2.version);
        assert_eq!(info1.platform, info2.platform);
        assert_eq!(info1.arch, info2.arch);
    }

    #[test]
    fn test_log_config_debug_format() {
        let config = LogConfig::new(Verbosity::Debug, true, false)
            .with_log_file(PathBuf::from("/tmp/test.log"));
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("LogConfig"));
        assert!(debug_str.contains("Debug"));
    }

    #[test]
    fn test_log_config_clone() {
        let config1 = LogConfig::new(Verbosity::Verbose, true, true)
            .with_log_file(PathBuf::from("/tmp/test.log"));
        let config2 = config1.clone();
        assert_eq!(config1.verbosity, config2.verbosity);
        assert_eq!(config1.json_output, config2.json_output);
        assert_eq!(config1.no_color, config2.no_color);
        assert_eq!(config1.log_file, config2.log_file);
    }
}
