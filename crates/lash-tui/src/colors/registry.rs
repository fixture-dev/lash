//! Color scheme registry for loading and looking up Gogh themes

use super::ColorScheme;
use once_cell::sync::Lazy;

/// Embedded Gogh themes JSON data
const THEMES_JSON: &str = include_str!("../../data/themes.json");

/// Global color scheme registry
///
/// Lazily initialized on first access. Contains all Gogh color schemes.
pub static REGISTRY: Lazy<SchemeRegistry> = Lazy::new(SchemeRegistry::default);

/// Registry for looking up color schemes by name
#[derive(Debug)]
pub struct SchemeRegistry {
    schemes: Vec<ColorScheme>,
}

impl Default for SchemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemeRegistry {
    /// Create a new registry by parsing embedded themes.json
    ///
    /// # Panics
    ///
    /// Panics if themes.json cannot be parsed. This should never happen in production
    /// since themes.json is validated at compile time.
    #[must_use]
    pub fn new() -> Self {
        let schemes: Vec<ColorScheme> =
            serde_json::from_str(THEMES_JSON).expect("Failed to parse embedded themes.json");

        Self { schemes }
    }

    /// Get all available scheme names, sorted alphabetically
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::colors::REGISTRY;
    ///
    /// let names = REGISTRY.scheme_names();
    /// assert!(names.len() > 200);
    /// assert!(names.contains(&"Base2Tone Desert".to_string()));
    /// ```
    #[must_use]
    pub fn scheme_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.schemes.iter().map(|s| s.name.clone()).collect();
        names.sort();
        names
    }

    /// Get a color scheme by exact name
    ///
    /// # Arguments
    ///
    /// * `name` - Exact scheme name (case-insensitive)
    ///
    /// # Returns
    ///
    /// Color scheme if found, None otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::colors::REGISTRY;
    ///
    /// let scheme = REGISTRY.get_scheme("Base2Tone Desert");
    /// assert!(scheme.is_some());
    ///
    /// let scheme = REGISTRY.get_scheme("base2tone desert"); // case-insensitive
    /// assert!(scheme.is_some());
    /// ```
    #[must_use]
    pub fn get_scheme(&self, name: &str) -> Option<&ColorScheme> {
        let name_lower = name.to_lowercase();
        self.schemes
            .iter()
            .find(|s| s.name.to_lowercase() == name_lower)
    }

    /// Get a color scheme by name, or return the default scheme
    ///
    /// Default scheme is `Base2Tone Desert`.
    ///
    /// # Arguments
    ///
    /// * `name` - Scheme name (case-insensitive)
    ///
    /// # Panics
    ///
    /// Panics if the default scheme `Base2Tone Desert` is not found in the registry.
    /// This should never happen in production as the themes are embedded at compile time.
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::colors::REGISTRY;
    ///
    /// let scheme = REGISTRY.get_scheme_or_default("Nonexistent");
    /// assert_eq!(scheme.name, "Base2Tone Desert");
    /// ```
    #[must_use]
    pub fn get_scheme_or_default(&self, name: &str) -> &ColorScheme {
        self.get_scheme(name)
            .or_else(|| self.get_scheme("Base2Tone Desert"))
            .expect("Default scheme 'Base2Tone Desert' not found in registry")
    }

    /// Find schemes matching a fuzzy search query
    ///
    /// Returns schemes whose names contain the query string (case-insensitive).
    /// Results are sorted by relevance (exact match first, then alphabetically).
    ///
    /// # Arguments
    ///
    /// * `query` - Search query
    ///
    /// # Returns
    ///
    /// Vector of matching scheme names
    ///
    /// # Examples
    ///
    /// ```
    /// use lash_tui::colors::REGISTRY;
    ///
    /// let matches = REGISTRY.fuzzy_search("desert");
    /// assert!(matches.iter().any(|s| s.contains("Desert")));
    /// ```
    #[must_use]
    pub fn fuzzy_search(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();

        let mut matches: Vec<String> = self
            .schemes
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&query_lower))
            .map(|s| s.name.clone())
            .collect();

        // Sort: exact match first, then alphabetically
        matches.sort_by(|a, b| {
            let a_lower = a.to_lowercase();
            let b_lower = b.to_lowercase();

            if a_lower == query_lower {
                std::cmp::Ordering::Less
            } else if b_lower == query_lower {
                std::cmp::Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        matches
    }

    /// Get the total number of schemes in the registry
    #[must_use]
    pub fn len(&self) -> usize {
        self.schemes.len()
    }

    /// Check if the registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.schemes.is_empty()
    }

    /// Get an iterator over all schemes
    pub fn iter(&self) -> impl Iterator<Item = &ColorScheme> {
        self.schemes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_loads() {
        let registry = SchemeRegistry::new();
        assert!(
            registry.len() > 200,
            "Expected 200+ schemes, got {}",
            registry.len()
        );
    }

    #[test]
    fn test_get_scheme() {
        let registry = SchemeRegistry::new();

        // Test exact match
        let scheme = registry.get_scheme("Base2Tone Desert");
        assert!(scheme.is_some());
        assert_eq!(scheme.unwrap().name, "Base2Tone Desert");

        // Test case-insensitive
        let scheme = registry.get_scheme("base2tone desert");
        assert!(scheme.is_some());

        // Test nonexistent
        let scheme = registry.get_scheme("Nonexistent Theme 12345");
        assert!(scheme.is_none());
    }

    #[test]
    fn test_get_scheme_or_default() {
        let registry = SchemeRegistry::new();

        // Test nonexistent falls back to default
        let scheme = registry.get_scheme_or_default("Nonexistent");
        assert_eq!(scheme.name, "Base2Tone Desert");

        // Test valid scheme returns that scheme
        let scheme = registry.get_scheme_or_default("3024 Night");
        assert_eq!(scheme.name, "3024 Night");
    }

    #[test]
    fn test_fuzzy_search() {
        let registry = SchemeRegistry::new();

        let matches = registry.fuzzy_search("desert");
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|s| s.to_lowercase().contains("desert")));

        // Test empty query returns nothing
        let matches = registry.fuzzy_search("xyznonexistent");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_scheme_names_sorted() {
        let registry = SchemeRegistry::new();
        let names = registry.scheme_names();

        assert!(!names.is_empty());

        // Check that names are sorted
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(names, sorted_names);
    }

    #[test]
    fn test_global_registry() {
        // Test that the global registry can be accessed
        let names = REGISTRY.scheme_names();
        assert!(names.len() > 200);
    }
}
