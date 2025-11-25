//! Shared types and error definitions for Lash
//!
//! This crate provides core types used across all Lash components, including:
//! - Error types and diagnostic structures
//! - Configuration types
//! - Common data structures

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
// Allow large Result errors - our error type is intentionally rich with context
// This is acceptable for a CLI tool where errors are exceptional, not the hot path
#![allow(clippy::result_large_err)]
// Allow longer functions in error formatting - these are necessarily detailed
#![allow(clippy::too_many_lines)]
// Allow format_push_string - clearer in many formatting contexts
#![allow(clippy::format_push_string)]
// Allow uninlined format args for now - can be cleaned up later
#![allow(clippy::uninlined_format_args)]

pub mod config;
pub mod dependency;
pub mod error;
pub mod file;
pub mod formatter;
pub mod index;
pub mod label;
pub mod report;
pub mod status;
pub mod task;

pub use config::{ConfigBuilder, LashConfig, UserConfig};
pub use dependency::{
    make_full_id, parse_dependency_ref, parse_full_id, Dependency, DependencyKind, DependencyRef,
};
pub use error::{Diagnostic, LashError, Location, Result, Severity};
pub use file::{compute_hash, synthesize_file_id, FileMetadata, FileStatus, TaskFile};
pub use formatter::ErrorFormatter;
pub use index::{find_index_file, IndexEntry, IndexMetadata, RootIndex};
pub use label::{
    is_valid_label, merge_labels, normalize, parse_annotation_labels, parse_inline_labels, Label,
};
pub use report::{ErrorReport, GroupBy, ReportSummary};
pub use status::TaskStatus;
pub use task::{Task, TaskBuilder, TaskMetadata, TaskTree};
