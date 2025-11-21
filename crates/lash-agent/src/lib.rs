//! Agent integration and prompt generation for Lash
//!
//! This crate provides:
//! - LLM prompt generation
//! - Token minimization utilities
//! - Agent-friendly output formats

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_precision_loss)] // Acceptable for token estimation
#![allow(clippy::cast_possible_truncation)] // Acceptable for token estimation
#![allow(clippy::cast_sign_loss)] // Result is always positive
#![allow(clippy::must_use_candidate)] // Too many false positives for utility functions
#![allow(clippy::format_push_string)] // More readable than write!() for simple cases

pub mod prompt;
pub mod schema;
pub mod tokens;

// Re-export commonly used types
pub use prompt::{AgentPrompt, PromptBuilder, PromptConfig, PromptFormat};
pub use schema::{generate_schema, generate_schema_text};
pub use tokens::{estimate_tokens, truncate_to_budget};
