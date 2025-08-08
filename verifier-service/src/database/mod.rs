pub mod constants;
pub mod migrator;
pub mod models;
pub mod operations;
pub mod sql;

use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use tracing::info;

pub use migrator::run_migrations;

/// Create a new SQLx pool and run migrations
pub async fn init_pool(db_path: &str) -> Result<SqlitePool> {
    info!("Opening database at {:?}", db_path);

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
