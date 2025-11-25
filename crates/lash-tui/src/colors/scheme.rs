//! Color scheme definition matching Gogh format

use ratatui::style::Color;
use serde::Deserialize;

/// A terminal color scheme from the Gogh collection
///
/// Contains 16 ANSI colors plus background, foreground, and cursor colors.
/// Colors are stored as hex strings (e.g., "#FF0000") and converted to RGB on demand.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ColorScheme {
    /// Scheme name (e.g., `Base2Tone Desert`)
    pub name: String,

    /// Author name (if available)
    #[serde(default)]
    pub author: String,

    /// Variant (e.g., "light", "dark", or empty)
    #[serde(default)]
    pub variant: String,

    /// ANSI color 0 (black)
    pub color_01: String,
    /// ANSI color 1 (red)
    pub color_02: String,
    /// ANSI color 2 (green)
    pub color_03: String,
    /// ANSI color 3 (yellow)
    pub color_04: String,
    /// ANSI color 4 (blue)
    pub color_05: String,
    /// ANSI color 5 (magenta)
    pub color_06: String,
    /// ANSI color 6 (cyan)
    pub color_07: String,
    /// ANSI color 7 (white)
    pub color_08: String,

    /// ANSI color 8 (bright black)
    pub color_09: String,
    /// ANSI color 9 (bright red)
    pub color_10: String,
    /// ANSI color 10 (bright green)
    pub color_11: String,
    /// ANSI color 11 (bright yellow)
    pub color_12: String,
    /// ANSI color 12 (bright blue)
    pub color_13: String,
    /// ANSI color 13 (bright magenta)
    pub color_14: String,
    /// ANSI color 14 (bright cyan)
    pub color_15: String,
    /// ANSI color 15 (bright white)
    pub color_16: String,

    /// Background color
    pub background: String,
    /// Foreground (text) color
    pub foreground: String,
    /// Cursor color
    pub cursor: String,

    /// Hash (for verification)
    #[serde(default)]
    pub hash: String,
}

impl ColorScheme {
    /// Convert a hex color string to RGB color
    ///
    /// # Arguments
    ///
    /// * `hex` - Hex color string (e.g., "#FF0000" or "FF0000")
    ///
    /// # Returns
    ///
    /// RGB color, or white if parsing fails
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::colors::ColorScheme;
    /// use ratatui::style::Color;
    ///
    /// let color = ColorScheme::hex_to_rgb("#FF0000");
    /// assert_eq!(color, Color::Rgb(255, 0, 0));
    /// ```
    #[must_use]
    pub fn hex_to_rgb(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');

        if hex.len() != 6 {
            return Color::White;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

        Color::Rgb(r, g, b)
    }

    /// Get the background color as RGB
    #[must_use]
    pub fn bg_color(&self) -> Color {
        Self::hex_to_rgb(&self.background)
    }

    /// Get the foreground color as RGB
    #[must_use]
    pub fn fg_color(&self) -> Color {
        Self::hex_to_rgb(&self.foreground)
    }

    /// Get the cursor color as RGB
    #[must_use]
    pub fn cursor_color(&self) -> Color {
        Self::hex_to_rgb(&self.cursor)
    }

    /// Get ANSI color by index (0-15)
    ///
    /// # Arguments
    ///
    /// * `index` - Color index (0-15)
    ///
    /// # Returns
    ///
    /// RGB color, or white if index is out of range
    #[must_use]
    pub fn ansi_color(&self, index: u8) -> Color {
        let hex = match index {
            0 => &self.color_01,
            1 => &self.color_02,
            2 => &self.color_03,
            3 => &self.color_04,
            4 => &self.color_05,
            5 => &self.color_06,
            6 => &self.color_07,
            7 => &self.color_08,
            8 => &self.color_09,
            9 => &self.color_10,
            10 => &self.color_11,
            11 => &self.color_12,
            12 => &self.color_13,
            13 => &self.color_14,
            14 => &self.color_15,
            15 => &self.color_16,
            _ => return Color::White,
        };
        Self::hex_to_rgb(hex)
    }

    /// Check if this is a light variant scheme
    #[must_use]
    pub fn is_light(&self) -> bool {
        self.variant.to_lowercase() == "light"
    }

    /// Check if this is a dark variant scheme
    #[must_use]
    pub fn is_dark(&self) -> bool {
        self.variant.to_lowercase() == "dark"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(ColorScheme::hex_to_rgb("#FF0000"), Color::Rgb(255, 0, 0));
        assert_eq!(ColorScheme::hex_to_rgb("#00FF00"), Color::Rgb(0, 255, 0));
        assert_eq!(ColorScheme::hex_to_rgb("#0000FF"), Color::Rgb(0, 0, 255));
        assert_eq!(ColorScheme::hex_to_rgb("FFFFFF"), Color::Rgb(255, 255, 255));
        assert_eq!(ColorScheme::hex_to_rgb("000000"), Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_hex_to_rgb_invalid() {
        // Invalid lengths should return white
        assert_eq!(ColorScheme::hex_to_rgb("#FFF"), Color::White);
        assert_eq!(ColorScheme::hex_to_rgb("#FFFFFFF"), Color::White);
        assert_eq!(ColorScheme::hex_to_rgb(""), Color::White);
    }

    #[test]
    fn test_ansi_color() {
        let scheme = ColorScheme {
            name: "Test".to_string(),
            author: String::new(),
            variant: String::new(),
            color_01: "#000000".to_string(),
            color_02: "#FF0000".to_string(),
            color_03: "#00FF00".to_string(),
            color_04: "#FFFF00".to_string(),
            color_05: "#0000FF".to_string(),
            color_06: "#FF00FF".to_string(),
            color_07: "#00FFFF".to_string(),
            color_08: "#FFFFFF".to_string(),
            color_09: "#808080".to_string(),
            color_10: "#FF8080".to_string(),
            color_11: "#80FF80".to_string(),
            color_12: "#FFFF80".to_string(),
            color_13: "#8080FF".to_string(),
            color_14: "#FF80FF".to_string(),
            color_15: "#80FFFF".to_string(),
            color_16: "#C0C0C0".to_string(),
            background: "#000000".to_string(),
            foreground: "#FFFFFF".to_string(),
            cursor: "#00FF00".to_string(),
            hash: String::new(),
        };

        assert_eq!(scheme.ansi_color(0), Color::Rgb(0, 0, 0));
        assert_eq!(scheme.ansi_color(1), Color::Rgb(255, 0, 0));
        assert_eq!(scheme.ansi_color(2), Color::Rgb(0, 255, 0));
        assert_eq!(scheme.ansi_color(15), Color::Rgb(192, 192, 192));
        assert_eq!(scheme.ansi_color(99), Color::White); // Out of range
    }

    #[test]
    fn test_variant_detection() {
        let mut scheme = ColorScheme {
            name: "Test".to_string(),
            author: String::new(),
            variant: "light".to_string(),
            color_01: String::new(),
            color_02: String::new(),
            color_03: String::new(),
            color_04: String::new(),
            color_05: String::new(),
            color_06: String::new(),
            color_07: String::new(),
            color_08: String::new(),
            color_09: String::new(),
            color_10: String::new(),
            color_11: String::new(),
            color_12: String::new(),
            color_13: String::new(),
            color_14: String::new(),
            color_15: String::new(),
            color_16: String::new(),
            background: String::new(),
            foreground: String::new(),
            cursor: String::new(),
            hash: String::new(),
        };

        assert!(scheme.is_light());
        assert!(!scheme.is_dark());

        scheme.variant = "dark".to_string();
        assert!(!scheme.is_light());
        assert!(scheme.is_dark());

        scheme.variant = String::new();
        assert!(!scheme.is_light());
        assert!(!scheme.is_dark());
    }
}
