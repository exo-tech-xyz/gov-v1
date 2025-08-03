pub mod constants;
pub mod migrator;
pub mod models;
pub mod operations;
pub mod sql;

use anyhow::Result;
use rusqlite::Connection;
use tracing::info;

pub use migrator::run_migrations;

/// Database manager for the verifier service
pub struct Database {
    connection: Connection,
}

impl Database {
    /// Create a new database connection and run migrations
    pub fn new(db_path: &str) -> Result<Self> {
        info!("Initializing database at {:?}", db_path);

        let connection = Connection::open(db_path)?;

        // Enable foreign key constraints
        connection.execute("PRAGMA foreign_keys = ON", [])?;

        // Run migrations
        run_migrations(&connection)?;

        info!("Database initialized successfully");

        Ok(Database { connection })
    }

    /// Get a reference to the database connection
    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}
