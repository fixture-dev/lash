//! Color scheme support using Gogh themes
//!
//! This module provides support for 300+ terminal color schemes from the Gogh project.
//! Color schemes are loaded from embedded JSON data and can be selected by name.

mod registry;
mod scheme;
mod theme;

pub use registry::{SchemeRegistry, REGISTRY};
pub use scheme::ColorScheme;
pub use theme::Theme;
