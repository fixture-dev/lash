//! Linting rules organized by category
//!
//! This module contains all the linting rules for Lash task files,
//! organized into three categories:
//!
//! - **Syntax rules** - Formatting and structure validation
//! - **Semantic rules** - Logical consistency and correctness (TODO: Task #3)
//! - **Cross-file rules** - Inter-file dependencies and references (TODO: Task #4)

pub mod syntax;

// Re-export all syntax rules for convenience
pub use syntax::{
    AnnotationOrderRule, AnnotationSyntaxRule, CheckboxPatternRule, DepthLimitRule,
    HeaderStructureRule, IndentationRule, UnknownAnnotationRule,
};
