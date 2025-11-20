//! `SQLite` indexing and query layer for Lash
//!
//! This crate provides:
//! - `SQLite` schema management
//! - Indexing task files into the database
//! - Query interface for fast lookups

#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod connection;
pub mod error;
pub mod migrations;
pub mod repository;

pub use connection::{get_schema_version, init_database, open_database, set_schema_version};
pub use error::{DbError, DbResult};
pub use migrations::{run_migrations, CURRENT_SCHEMA_VERSION};
pub use repository::{DependencyRepository, FileRepository, LabelRepository, TaskRepository};
