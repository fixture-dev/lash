//! Agent integration and prompt generation for Lash
//!
//! This crate provides:
//! - LLM prompt generation
//! - Token minimization utilities
//! - Agent-friendly output formats

#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_precision_loss)] // Acceptable for token estimation
#![allow(clippy::cast_possible_truncation)] // Acceptable for token estimation
#![allow(clippy::cast_sign_loss)] // Result is always positive
#![allow(clippy::must_use_candidate)] // Too many false positives for utility functions
#![allow(clippy::format_push_string)] // More readable than write!() for simple cases

pub mod content;
pub mod context;
pub mod installer;
pub mod prompt;
pub mod schema;
pub mod tokens;

// Re-export commonly used types
pub use context::{ContextBuilder, ContextFormat, InclusionRules, SparseContext};
pub use prompt::{
    AgentPrompt, DocRefInfo, PromptBuilder, PromptConfig, PromptFormat, TaskFileSummary,
};
pub use schema::{
    generate_contextual_notes_example, generate_dependency_example, generate_doc_reference_example,
    generate_minimal_example, generate_schema, generate_schema_text,
};
pub use tokens::{estimate_tokens, truncate_to_budget};
