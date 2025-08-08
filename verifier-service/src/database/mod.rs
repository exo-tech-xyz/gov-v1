pub mod constants;
pub mod migrator;
pub mod models;
pub mod operations;
pub mod sql;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::{fs, path::Path};
use tracing::info;

pub use migrator::run_migrations;

/// Create a new SQLx pool and run migrations
pub async fn init_pool(db_path: &str) -> Result<SqlitePool> {
    info!("Opening database at {:?}", db_path);

    // Ensure parent directory exists
    if db_path != ":memory:" {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        // Create the DB file if it doesn't exist
        if !path.exists() {
            fs::File::create(path)?;
        }
    }

    // SQLite URL form for SQLx
    let db_url = if db_path == ":memory:" {
        "sqlite::memory:".to_string()
    } else {
        format!("sqlite:{}", db_path)
    };

    let pool = SqlitePool::connect(&db_url).await?;

    // Enable pragma settings for better concurrency
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    info!("Database pool initialized successfully");
    Ok(pool)
}
