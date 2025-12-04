//! Linting rules organized by category
//!
//! This module contains all the linting rules for Lash task files,
//! organized into three categories:
//!
//! - **Syntax rules** - Formatting and structure validation
//! - **Semantic rules** - Logical consistency and correctness
//! - **Cross-file rules** - Inter-file dependencies and references

pub mod crossfile;
pub mod semantic;
pub mod syntax;

// Re-export all syntax rules for convenience
pub use syntax::{
    AnnotationOrderRule, AnnotationSyntaxRule, CheckboxPatternRule, DepthLimitRule,
    DuplicateDescriptionRule, HeaderStructureRule, IndentationRule, UnknownAnnotationRule,
};

// Re-export all semantic rules for convenience
pub use semantic::{
    AutoWaiveRule, BrokenDocFragmentRule, DescriptionLengthRule, DuplicateIdRule, EmptyTitleRule,
    StatusConsistencyRule, ValidDateRule, ValidDocReferenceRule, ValidEstimateRule, ValidLabelRule,
    ValidOwnerRule,
};

// Re-export all cross-file rules for convenience
pub use crossfile::{
    CircularDepsRule, DependencyExistsRule, IndexFileRefsRule, OrphanedFilesRule,
    ValidPathResolutionRule,
};
