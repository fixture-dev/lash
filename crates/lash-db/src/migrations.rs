//! Database schema migrations

use rusqlite::Connection;

use crate::connection::{get_schema_version, set_schema_version};
use crate::error::{DbError, DbResult};

mod v2_enhanced_fts;
mod v3_doc_refs;
mod v4_file_descriptions;
mod v5_fts_description;
mod v6_contextual_notes;
mod v7_fts_contextual_notes;

use v2_enhanced_fts::MigrationV2EnhancedFts;
use v3_doc_refs::MigrationV3DocRefs;
use v4_file_descriptions::MigrationV4FileDescriptions;
use v5_fts_description::MigrationV5FtsDescription;
use v6_contextual_notes::MigrationV6ContextualNotes;
use v7_fts_contextual_notes::MigrationV7FtsContextualNotes;

/// Current schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 7;

/// A database migration
pub trait Migration {
    /// Get the version this migration targets
    fn version(&self) -> i32;

    /// Get a description of what this migration does
    fn description(&self) -> &str;

    /// Apply the migration
    ///
    /// # Errors
    ///
    /// Returns error if migration fails
    fn up(&self, conn: &Connection) -> DbResult<()>;

    /// Rollback the migration (optional, for future use)
    ///
    /// # Errors
    ///
    /// Returns error if rollback is not implemented or fails
    fn down(&self, _conn: &Connection) -> DbResult<()> {
        Err(DbError::Other(
            "Rollback not implemented for this migration".to_string(),
        ))
    }
}

/// Run all pending migrations
///
/// Checks the current schema version and applies any migrations needed to
/// bring the database up to the current version.
///
/// # Example
///
/// ```no_run
/// use lash_db::connection::init_database;
/// use lash_db::migrations::run_migrations;
/// use std::path::Path;
///
/// let conn = init_database(Path::new("/tmp/lash.db")).unwrap();
/// run_migrations(&conn).unwrap();
/// ```
///
/// # Errors
///
/// Returns error if:
/// - Current schema version cannot be read
/// - Any migration fails to apply
/// - Schema version update fails
pub fn run_migrations(conn: &Connection) -> DbResult<()> {
    let current_version = get_schema_version(conn)?;

    if current_version > CURRENT_SCHEMA_VERSION {
        return Err(DbError::SchemaMismatch {
            expected: CURRENT_SCHEMA_VERSION,
            found: current_version,
        });
    }

    if current_version == CURRENT_SCHEMA_VERSION {
        // Already up to date
        return Ok(());
    }

    // Get all migrations that need to be applied
    let migrations = get_migrations();
    let pending: Vec<_> = migrations
        .iter()
        .filter(|m| m.version() > current_version)
        .collect();

    if pending.is_empty() {
        return Ok(());
    }

    // Apply migrations in order
    for migration in pending {
        apply_migration(conn, migration.as_ref())?;
    }

    Ok(())
}

/// Apply a single migration
fn apply_migration(conn: &Connection, migration: &dyn Migration) -> DbResult<()> {
    let version = migration.version();

    // Use a transaction for safety
    let tx = conn.unchecked_transaction()?;

    migration.up(&tx).map_err(|e| DbError::MigrationFailed {
        version,
        reason: e.to_string(),
    })?;

    set_schema_version(&tx, version)?;

    tx.commit()?;

    Ok(())
}

/// Get all available migrations in order
fn get_migrations() -> Vec<Box<dyn Migration>> {
    vec![
        Box::new(MigrationV2EnhancedFts),
        Box::new(MigrationV3DocRefs),
        Box::new(MigrationV4FileDescriptions),
        Box::new(MigrationV5FtsDescription),
        Box::new(MigrationV6ContextualNotes),
        Box::new(MigrationV7FtsContextualNotes),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::init_database;
    use tempfile::NamedTempFile;

    #[test]
    fn test_run_migrations_on_current_version() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Should be a no-op since we're already at current version
        run_migrations(&conn).unwrap();

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_detect_future_schema_version() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();

        // Set version to future version
        set_schema_version(&conn, 999).unwrap();

        // Should fail with schema mismatch
        let result = run_migrations(&conn);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DbError::SchemaMismatch { .. }
        ));
    }
}
