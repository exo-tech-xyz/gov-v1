//! Database migration constants and metadata

/// Current database schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Migration descriptions
pub const MIGRATION_DESCRIPTIONS: &[&str] = &["Initial schema with network support"];

/// Default database file name
pub const DATABASE_FILE: &str = "governance.db";
