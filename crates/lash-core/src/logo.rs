//! Lash branding and logo utilities
//!
//! This module provides the Lash terminal logo for display in CLI and TUI contexts.

/// The Lash ASCII logo for terminal display.
///
/// This is a compact 3-line ASCII art logo using box-drawing characters.
pub const LOGO: &str = "\
┓    ┓
┃ ┏┓┏┣┓
┗┛┗┻┛┛┗";

/// Individual lines of the logo for flexible rendering.
pub const LOGO_LINES: [&str; 3] = ["┓    ┓", "┃ ┏┓┏┣┓", "┗┛┗┻┛┛┗"];

/// The width of the logo in characters.
pub const LOGO_WIDTH: usize = 8;

/// The height of the logo in lines.
pub const LOGO_HEIGHT: usize = 3;

/// Returns the logo as a string with an optional trailing newline.
///
/// # Arguments
///
/// * `trailing_newline` - Whether to append a newline at the end
///
/// # Examples
///
/// ```
/// use lash_core::logo::get_logo;
///
/// let logo = get_logo(false);
/// assert!(!logo.ends_with('\n'));
///
/// let logo_with_newline = get_logo(true);
/// assert!(logo_with_newline.ends_with('\n'));
/// ```
#[must_use]
pub fn get_logo(trailing_newline: bool) -> String {
    if trailing_newline {
        format!("{LOGO}\n")
    } else {
        LOGO.to_string()
    }
}

/// Returns the logo formatted for CLI help output.
///
/// Includes the logo followed by a blank line for visual separation.
///
/// # Examples
///
/// ```
/// use lash_core::logo::logo_for_help;
///
/// let help_logo = logo_for_help();
/// assert!(help_logo.contains("┏┓┏┣┓"));
/// assert!(help_logo.ends_with("\n\n"));
/// ```
#[must_use]
pub fn logo_for_help() -> String {
    format!("{LOGO}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logo_dimensions() {
        let lines: Vec<&str> = LOGO.lines().collect();
        assert_eq!(lines.len(), LOGO_HEIGHT);

        // Verify each line matches the LOGO_LINES constant
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(*line, LOGO_LINES[i]);
        }
    }

    #[test]
    fn test_get_logo_without_newline() {
        let logo = get_logo(false);
        assert!(!logo.ends_with('\n'));
        assert_eq!(logo, LOGO);
    }

    #[test]
    fn test_get_logo_with_newline() {
        let logo = get_logo(true);
        assert!(logo.ends_with('\n'));
        assert_eq!(logo.trim_end(), LOGO);
    }

    #[test]
    fn test_logo_for_help() {
        let help_logo = logo_for_help();
        assert!(help_logo.ends_with("\n\n"));
        assert!(help_logo.contains(LOGO));
    }
}
