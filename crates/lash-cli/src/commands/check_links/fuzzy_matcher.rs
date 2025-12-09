//! Fuzzy matching for finding similar task IDs
//!
//! This module re-exports the fuzzy matching functionality from `lash_core`.
//! The implementation has been moved to `lash_core::fuzzy` to allow reuse
//! across multiple crates (e.g., lash-tui for autocomplete).

// Re-export from lash-core
pub use lash_core::fuzzy::{FuzzyCandidate, FuzzyMatcher};
