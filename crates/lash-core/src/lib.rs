//! Core parsing, validation, and task model for Lash
//!
//! This crate provides:
//! - Markdown parsing for task files
//! - Task data model and validation
//! - Linter implementation
//! - Dependency resolution

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// Parser module - implemented
pub mod parser;

// Linter module - Task #1 (infrastructure) implemented
pub mod linter;

// Module placeholders - will be implemented in subsequent tasks
// pub mod validator;
// pub mod dependency;
