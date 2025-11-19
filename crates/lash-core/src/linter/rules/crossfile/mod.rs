//! Cross-file validation rules
//!
//! This module contains rules that validate relationships between files,
//! including:
//!
//! - Dependency reference validation (file and task references exist)
//! - Circular dependency detection
//! - Root index file validation
//! - Orphaned file detection
//! - Path resolution validation

mod circular_deps;
mod dependency_exists;
mod index_file_refs;
mod orphaned_files;
mod valid_path_resolution;

pub use circular_deps::CircularDepsRule;
pub use dependency_exists::DependencyExistsRule;
pub use index_file_refs::IndexFileRefsRule;
pub use orphaned_files::OrphanedFilesRule;
pub use valid_path_resolution::ValidPathResolutionRule;
