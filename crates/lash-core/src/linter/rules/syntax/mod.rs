//! Syntax validation rules
//!
//! This module contains rules that validate the syntax and formatting of
//! Lash Markdown files. Syntax rules ensure consistent, predictable file
//! structure that both humans and AI agents can easily parse and understand.
//!
//! # Implementation Note
//!
//! Some syntax rules require access to the raw markdown content because they
//! validate aspects that are normalized away during parsing:
//!
//! - **Checkbox pattern** - Parser only accepts valid patterns
//! - **Indentation** - Normalized to depth in parsed structure
//! - **Annotation syntax** - Parser only accepts valid syntax
//! - **Annotation order** - Lost during parsing into structured fields
//!
//! For these rules, there are two implementation approaches:
//!
//! 1. **Parser-level validation** - Emit diagnostics during parsing (currently done)
//! 2. **Raw content rules** - Add a `check_raw_content` method to `LintRule` trait
//!
//! For now, we implement rules that work with the parsed `TaskFile` structure.
//! Parser-level validation already catches most syntax errors. Additional
//! post-parse rules could be added if needed with raw content access.

pub mod annotation_order;
pub mod annotation_syntax;
pub mod checkbox_pattern;
pub mod depth_limit;
pub mod duplicate_description;
pub mod header_structure;
pub mod indentation;
pub mod unknown_annotation;

pub use annotation_order::AnnotationOrderRule;
pub use annotation_syntax::AnnotationSyntaxRule;
pub use checkbox_pattern::CheckboxPatternRule;
pub use depth_limit::DepthLimitRule;
pub use duplicate_description::DuplicateDescriptionRule;
pub use header_structure::HeaderStructureRule;
pub use indentation::IndentationRule;
pub use unknown_annotation::UnknownAnnotationRule;
