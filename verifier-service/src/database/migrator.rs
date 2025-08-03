//! Database migration implementation

use anyhow::Result;
use rusqlite::{params, Connection};
use tracing::info;

use super::constants::MIGRATION_DESCRIPTIONS;
use super::sql::{
    CREATE_DB_INDEXES, CREATE_MIGRATIONS_TABLE_SQL, CREATE_SNAPSHOT_META_TABLE_SQL,
    CREATE_STAKE_ACCOUNTS_TABLE_SQL, CREATE_VOTE_ACCOUNTS_TABLE_SQL,
};

/// Run all pending database migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    info!("Running database migrations");

    // Create migrations table first
    create_migrations_table(conn)?;

    // Get current version
    let current_version = get_current_version(conn)?;
    info!("Current database version: {}", current_version);

    // Apply migrations in order
    if current_version < 1 {
        apply_migration_v1(conn)?;
    }

    info!("All migrations completed");
    Ok(())
}

/// Create the schema_migrations table
fn create_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute(CREATE_MIGRATIONS_TABLE_SQL, [])?;
    Ok(())
}

/// Get the current schema version
fn get_current_version(conn: &Connection) -> Result<i32> {
    let mut stmt = conn.prepare("SELECT MAX(version) FROM schema_migrations")?;
    let version: Option<i32> = stmt.query_row([], |row| row.get(0)).unwrap_or(None);
    Ok(version.unwrap_or(0))
}

/// Apply migration version 1: Initiate tables and indexes.
fn apply_migration_v1(conn: &Connection) -> Result<()> {
    info!("Applying migration v1: {}", MIGRATION_DESCRIPTIONS[0]);

    let tx = conn.unchecked_transaction()?;

    // Create core tables and indexes
    tx.execute(CREATE_VOTE_ACCOUNTS_TABLE_SQL, [])?;
    tx.execute(CREATE_STAKE_ACCOUNTS_TABLE_SQL, [])?;
    tx.execute(CREATE_SNAPSHOT_META_TABLE_SQL, [])?;

    for index_sql in CREATE_DB_INDEXES {
        tx.execute(index_sql, [])?;
    }

    // Record migration
    tx.execute(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?, ?, ?)",
        params![
            1,
            chrono::Utc::now().to_rfc3339(),
            MIGRATION_DESCRIPTIONS[0]
        ],
    )?;

    tx.commit()?;

    info!("Migration v1 completed successfully");
    Ok(())
}
