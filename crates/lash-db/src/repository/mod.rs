//! Repository layer for database access
//!
//! Provides high-level CRUD operations and queries for:
//! - Files
//! - Tasks
//! - Dependencies
//! - Labels

pub mod dependencies;
pub mod files;
pub mod labels;
pub mod tasks;

pub use dependencies::DependencyRepository;
pub use files::{normalize_path_for_db, FileRepository};
pub use labels::LabelRepository;
pub use tasks::TaskRepository;
