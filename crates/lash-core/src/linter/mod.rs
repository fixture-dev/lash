//! Linting and validation for Lash Markdown files
//!
//! The linter enforces both syntax and semantic correctness for Lash task files.
//! It uses a rule-based architecture where each rule is independent and can be
//! enabled/disabled through configuration.
//!
//! Note: The `linter` submodule contains the main linting implementation.

#![allow(clippy::module_inception)]
//!
//! # Architecture
//!
//! The linter is built around the `LintRule` trait, which defines a common
//! interface for all validation rules. Rules can operate at the file level
//! or task level, and can provide auto-fix suggestions where appropriate.
//!
//! ```text
//! LintRule (trait)
//!     ↓
//! Concrete Rules (20+)
//!     ↓
//! Linter (orchestrates rules)
//!     ↓
//! Diagnostic (reports issues)
//!     ↓
//! Fix (auto-fix suggestions)
//! ```
//!
//! # Example
//!
//! ```
//! use lash_core::linter::{Linter, LintConfig};
//! use lash_types::{LashConfig, TaskFile, FileMetadata, TaskTree};
//! use std::path::PathBuf;
//! use std::time::SystemTime;
//!
//! let config = LintConfig::default();
//! let linter = Linter::new(config);
//!
//! // Create a simple task file for demonstration
//! let file = TaskFile {
//!     path: PathBuf::from("test.md"),
//!     title: "Test".to_string(),
//!     id: "test".to_string(),
//!     metadata: FileMetadata::default(),
//!     description: None,
//!     description_agent_notes: Vec::new(),
//!     tasks: TaskTree::new(),
//!     hash: "hash".to_string(),
//!     mtime: SystemTime::now(),
//! };
//!
//! let project_config = LashConfig::default();
//! let diagnostics = linter.lint_file(&file, &project_config);
//! assert_eq!(diagnostics.len(), 0);
//! ```

pub mod config;
pub mod context;
pub mod diagnostic;
pub mod fix;
pub mod fix_applicator;
pub mod linter;
pub mod registry;
pub mod rule;
pub mod rules;

pub use config::LintConfig;
pub use context::LintContext;
pub use diagnostic::LintDiagnostic;
pub use fix::{Fix, Replacement};
pub use fix_applicator::{ApplyResult, FixApplicator, SkippedFix};
pub use linter::Linter;
pub use registry::{register_default_rules, RuleCategory, RuleRegistry};
pub use rule::LintRule;
