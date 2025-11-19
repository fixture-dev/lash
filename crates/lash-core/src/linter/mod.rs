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
//! ```rust,ignore
//! use lash_core::linter::{Linter, LintConfig};
//! use lash_types::TaskFile;
//!
//! let config = LintConfig::default();
//! let linter = Linter::new(config);
//! let file: TaskFile = /* parse file */;
//!
//! let diagnostics = linter.lint_file(&file);
//! for diagnostic in diagnostics {
//!     println!("{}", diagnostic);
//! }
//! ```

pub mod config;
pub mod context;
pub mod diagnostic;
pub mod fix;
pub mod linter;
pub mod registry;
pub mod rule;

pub use config::LintConfig;
pub use context::LintContext;
pub use diagnostic::LintDiagnostic;
pub use fix::{Fix, Replacement};
pub use linter::Linter;
pub use registry::{register_default_rules, RuleCategory, RuleRegistry};
pub use rule::LintRule;
