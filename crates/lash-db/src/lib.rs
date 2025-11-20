//! `SQLite` indexing and query layer for Lash
//!
//! This crate provides:
//! - `SQLite` schema management
//! - Indexing task files into the database
//! - Query interface for fast lookups

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod connection;
pub mod dependency_updater;
pub mod diff;
pub mod error;
pub mod indexer;
pub mod migrations;
pub mod project_root;
pub mod repository;
pub mod verifier;
pub mod walker;

pub use connection::{get_schema_version, init_database, open_database, set_schema_version};
pub use dependency_updater::DependencyUpdater;
pub use diff::{compute_index_diff, compute_index_diff_parallel, IndexDiff};
pub use error::{DbError, DbResult};
pub use indexer::{IndexProgress, IndexReport, Indexer, IndexerConfig, ParseError};
pub use migrations::{run_migrations, CURRENT_SCHEMA_VERSION};
pub use project_root::{
    find_project_root, find_project_root_from, find_project_root_with_config, is_project_root,
    ProjectRootConfig,
};
pub use repository::{DependencyRepository, FileRepository, LabelRepository, TaskRepository};
pub use verifier::{
    IndexVerifier, IssueKind, VerificationIssue, VerificationReport, VerifierConfig,
};
pub use walker::{FileMetadata, FileWalker, FileWalkerConfig};
