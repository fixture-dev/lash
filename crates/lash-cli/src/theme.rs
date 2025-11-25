//! CLI theme module wrapping lash-tui Theme for terminal color styling
//!
//! This module provides a CLI-friendly wrapper around the lash-tui Theme, converting
//! ratatui colors to owo-colors styles for terminal output. It handles theme loading
//! from multiple sources with priority: CLI args > user config > default.

use lash_tui::colors::{Theme as TuiTheme, REGISTRY};
use lash_types::{TaskStatus, UserConfig};
use owo_colors::{OwoColorize, Stream};
use ratatui::style::Color;

// Re-export the TUI theme for direct access if needed
pub use lash_tui::colors::Theme;

/// CLI-friendly theme wrapper providing terminal color styling
///
/// Wraps the lash-tui Theme and provides methods to style text using owo-colors
/// for terminal output. Handles color-disabled scenarios gracefully.
///
/// # Examples
///
/// ```
/// use lash_cli::theme::CliTheme;
///
/// if let Some(theme) = CliTheme::load(None, true).unwrap() {
///     let success = theme.style_success("Operation completed");
///     println!("{}", success);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CliTheme {
    /// Underlying TUI theme
    theme: TuiTheme,
    /// Whether colors are enabled
    colors_enabled: bool,
}

impl CliTheme {
    /// Create a new CLI theme wrapper
    ///
    /// # Arguments
    ///
    /// * `theme` - The underlying TUI theme
    /// * `colors_enabled` - Whether to apply colors to output
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let cli_theme = CliTheme::new(tui_theme, true);
    /// ```
    #[must_use]
    pub fn new(theme: TuiTheme, colors_enabled: bool) -> Self {
        Self {
            theme,
            colors_enabled,
        }
    }

    /// Load a theme based on priority: CLI arg > user config > default
    ///
    /// # Arguments
    ///
    /// * `scheme_name` - Optional color scheme name from CLI argument
    /// * `colors_enabled` - Whether colors should be enabled (respects `NO_COLOR`, --no-color, etc.)
    ///
    /// # Returns
    ///
    /// A CLI theme, or None if colors are explicitly disabled
    ///
    /// # Errors
    ///
    /// Returns an error if user config cannot be loaded or scheme name is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lash_cli::theme::CliTheme;
    ///
    /// // Load from user config or default
    /// let theme = CliTheme::load(None, true)?;
    ///
    /// // Override with specific scheme
    /// let theme = CliTheme::load(Some("Dracula"), true)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn load(scheme_name: Option<&str>, colors_enabled: bool) -> anyhow::Result<Option<Self>> {
        // If colors are explicitly disabled, return None
        if !colors_enabled {
            return Ok(None);
        }

        // Determine which scheme to use - convert to owned String to avoid lifetime issues
        let scheme_name = match scheme_name {
            Some(name) => name.to_string(),
            None => {
                // Try to load from user config
                match UserConfig::load() {
                    Ok(config) => config.color_scheme,
                    Err(_) => {
                        // Fall back to default if user config can't be loaded
                        "Base2Tone Desert".to_string()
                    }
                }
            }
        };

        // Load the scheme from the registry
        let scheme = REGISTRY
            .get_scheme(&scheme_name)
            .ok_or_else(|| anyhow::anyhow!("Color scheme '{scheme_name}' not found"))?;

        let theme = TuiTheme::new(scheme.clone());
        Ok(Some(Self::new(theme, true)))
    }

    /// Get the underlying TUI theme
    #[must_use]
    pub fn tui_theme(&self) -> &TuiTheme {
        &self.theme
    }

    /// Check if colors are enabled
    #[must_use]
    pub fn has_colors(&self) -> bool {
        self.colors_enabled
    }

    /// Get the theme name
    #[must_use]
    pub fn name(&self) -> &str {
        self.theme.name()
    }

    /// Convert a ratatui Color to an owo-colors RGB style
    fn color_to_style(color: Color) -> owo_colors::Style {
        match color {
            Color::Rgb(r, g, b) => owo_colors::Style::new().color(owo_colors::Rgb(r, g, b)),
            // For other color types (including Reset), use a default style
            _ => owo_colors::Style::new(),
        }
    }

    /// Style text with a given color, or return unstyled if colors disabled
    fn style_text(&self, text: &str, color: Color) -> String {
        if !self.colors_enabled {
            return text.to_string();
        }

        let style = Self::color_to_style(color);
        text.if_supports_color(Stream::Stdout, |t| t.style(style))
            .to_string()
    }

    /// Style text as success (green)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let text = theme.style_success("All tests passed");
    /// # assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub fn style_success(&self, text: &str) -> String {
        self.style_text(text, self.theme.success_color())
    }

    /// Style text as error (red)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let text = theme.style_error("Failed to load file");
    /// # assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub fn style_error(&self, text: &str) -> String {
        self.style_text(text, self.theme.error_color())
    }

    /// Style text as warning (yellow)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let text = theme.style_warning("Deprecated feature");
    /// # assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub fn style_warning(&self, text: &str) -> String {
        self.style_text(text, self.theme.warning_color())
    }

    /// Style text as info (blue)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let text = theme.style_info("Processing 10 files");
    /// # assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub fn style_info(&self, text: &str) -> String {
        self.style_text(text, self.theme.info_color())
    }

    /// Style text as muted/secondary (gray)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let text = theme.style_muted("(optional)");
    /// # assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub fn style_muted(&self, text: &str) -> String {
        self.style_text(text, self.theme.muted_color())
    }

    /// Style text as a label/tag (cyan)
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let text = theme.style_label("#backend");
    /// # assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub fn style_label(&self, text: &str) -> String {
        self.style_text(text, self.theme.label_color())
    }

    /// Style text based on task status
    ///
    /// # Arguments
    ///
    /// * `text` - The text to style
    /// * `status` - The task status determining the color
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    /// use lash_types::TaskStatus;
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let done = theme.style_task_status("[x]", TaskStatus::Done);
    /// let blocked = theme.style_task_status("[!]", TaskStatus::Blocked);
    /// # assert!(!done.is_empty());
    /// # assert!(!blocked.is_empty());
    /// ```
    #[must_use]
    pub fn style_task_status(&self, text: &str, status: TaskStatus) -> String {
        let color = match status {
            TaskStatus::Done => self.theme.task_done(),
            TaskStatus::Blocked => self.theme.task_blocked(),
            TaskStatus::Open => self.theme.task_open(),
            TaskStatus::Waived => self.theme.task_waived(),
        };
        self.style_text(text, color)
    }

    /// Get the checkbox character for a task status with styling
    ///
    /// Returns the styled checkbox character like `[x]`, `[!]`, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_cli::theme::CliTheme;
    /// use lash_tui::colors::{Theme, REGISTRY};
    /// use lash_types::TaskStatus;
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
    /// let tui_theme = Theme::new(scheme.clone());
    /// let theme = CliTheme::new(tui_theme, true);
    ///
    /// let checkbox = theme.styled_checkbox(TaskStatus::Done);
    /// # assert!(checkbox.contains("x"));
    /// ```
    #[must_use]
    pub fn styled_checkbox(&self, status: TaskStatus) -> String {
        let checkbox = self.theme.checkbox_char(status);
        self.style_task_status(checkbox, status)
    }
}

/// Check if the terminal supports color output
///
/// Returns false if:
/// - `NO_COLOR` environment variable is set
/// - Stdout is not a TTY
/// - Otherwise returns true
///
/// # Examples
///
/// ```
/// use lash_cli::theme::supports_color;
///
/// if supports_color() {
///     println!("Colors are supported");
/// }
/// ```
#[must_use]
pub fn supports_color() -> bool {
    // NO_COLOR environment variable takes precedence
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // Check if stdout is a TTY
    atty::is(atty::Stream::Stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> CliTheme {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = TuiTheme::new(scheme.clone());
        CliTheme::new(theme, true)
    }

    #[test]
    fn test_cli_theme_creation() {
        let theme = test_theme();
        assert!(theme.has_colors());
        assert_eq!(theme.name(), "Base2Tone Desert");
    }

    #[test]
    fn test_cli_theme_no_colors() {
        let scheme = REGISTRY.get_scheme("Base2Tone Desert").unwrap();
        let theme = TuiTheme::new(scheme.clone());
        let cli_theme = CliTheme::new(theme, false);

        assert!(!cli_theme.has_colors());

        // With colors disabled, should return unstyled text
        let text = "test";
        assert_eq!(cli_theme.style_success(text), text);
        assert_eq!(cli_theme.style_error(text), text);
        assert_eq!(cli_theme.style_warning(text), text);
        assert_eq!(cli_theme.style_info(text), text);
        assert_eq!(cli_theme.style_muted(text), text);
        assert_eq!(cli_theme.style_label(text), text);
    }

    #[test]
    fn test_semantic_styling() {
        let theme = test_theme();

        // These should not panic and should return non-empty strings
        let success = theme.style_success("success");
        let error = theme.style_error("error");
        let warning = theme.style_warning("warning");
        let info = theme.style_info("info");
        let muted = theme.style_muted("muted");
        let label = theme.style_label("#label");

        assert!(!success.is_empty());
        assert!(!error.is_empty());
        assert!(!warning.is_empty());
        assert!(!info.is_empty());
        assert!(!muted.is_empty());
        assert!(!label.is_empty());
    }

    #[test]
    fn test_task_status_styling() {
        let theme = test_theme();

        let done = theme.style_task_status("done", TaskStatus::Done);
        let blocked = theme.style_task_status("blocked", TaskStatus::Blocked);
        let open = theme.style_task_status("open", TaskStatus::Open);
        let waived = theme.style_task_status("waived", TaskStatus::Waived);

        assert!(!done.is_empty());
        assert!(!blocked.is_empty());
        assert!(!open.is_empty());
        assert!(!waived.is_empty());
    }

    #[test]
    fn test_styled_checkbox() {
        let theme = test_theme();

        let done = theme.styled_checkbox(TaskStatus::Done);
        let blocked = theme.styled_checkbox(TaskStatus::Blocked);
        let open = theme.styled_checkbox(TaskStatus::Open);
        let waived = theme.styled_checkbox(TaskStatus::Waived);

        // Verify checkboxes contain expected characters
        assert!(done.contains('x'));
        assert!(blocked.contains('!'));
        assert!(open.contains(' '));
        assert!(waived.contains('-'));
    }

    #[test]
    fn test_load_default() {
        // This test requires NO_COLOR to be unset
        std::env::remove_var("NO_COLOR");

        let result = CliTheme::load(None, true);
        assert!(result.is_ok());

        if let Ok(Some(theme)) = result {
            // Should load default theme
            assert!(theme.has_colors());
        }
    }

    #[test]
    fn test_load_specific_scheme() {
        let result = CliTheme::load(Some("3024 Night"), true);
        assert!(result.is_ok());

        if let Ok(Some(theme)) = result {
            assert_eq!(theme.name(), "3024 Night");
        }
    }

    #[test]
    fn test_load_invalid_scheme() {
        let result = CliTheme::load(Some("Nonexistent Scheme 12345"), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_colors_disabled() {
        let result = CliTheme::load(None, false);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_color_to_style() {
        // Test RGB color conversion
        let style = CliTheme::color_to_style(Color::Rgb(255, 0, 0));
        // Style should be created without panicking
        let _ = "test".style(style);

        // Test Reset color
        let style = CliTheme::color_to_style(Color::Reset);
        let _ = "test".style(style);
    }

    #[test]
    fn test_tui_theme_accessor() {
        let theme = test_theme();
        let tui = theme.tui_theme();
        assert_eq!(tui.name(), "Base2Tone Desert");
    }
}
