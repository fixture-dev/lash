//! Shared types and error definitions for Lash
//!
//! This crate provides core types used across all Lash components, including:
//! - Error types and diagnostic structures
//! - Configuration types
//! - Common data structures

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod dependency;
pub mod error;
pub mod file;
pub mod index;
pub mod label;
pub mod status;
pub mod task;

pub use config::{ConfigBuilder, LashConfig};
pub use dependency::{
    make_full_id, parse_dependency_ref, parse_full_id, Dependency, DependencyKind, DependencyRef,
};
pub use error::{Diagnostic, LashError, Location, Result, Severity};
pub use file::{compute_hash, synthesize_file_id, FileMetadata, FileStatus, TaskFile};
pub use index::{find_index_file, IndexEntry, IndexMetadata, RootIndex};
pub use label::{
    is_valid_label, merge_labels, normalize, parse_annotation_labels, parse_inline_labels, Label,
};
pub use status::TaskStatus;
pub use task::{Task, TaskBuilder, TaskMetadata, TaskTree};
