//! Shared types and error definitions for Lash
//!
//! This crate provides core types used across all Lash components, including:
//! - Error types and diagnostic structures
//! - Configuration types
//! - Common data structures

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod error;

pub use config::{ConfigBuilder, LashConfig};
pub use error::{Diagnostic, LashError, Location, Result, Severity};
