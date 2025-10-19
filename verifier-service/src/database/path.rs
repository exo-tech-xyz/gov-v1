use anyhow::{bail, Result};
use std::{fs, path::Path};

pub fn validate_db_path(db_path: &str) -> Result<()> {
    if db_path == ":memory:" {
        return Ok(());
    }

    if db_path.is_empty() {
        bail!("Empty database path");
    }

    if db_path.contains('\0') || db_path.contains(['\n', '\r', '\t']) {
        bail!("Invalid control characters in database path");
    }

    let path = Path::new(db_path);

    // Disallow parent directory traversal components
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            bail!("Parent directory traversal is not allowed in database path");
        }
    }

    // Require a terminal file name (avoid paths ending with a directory separator)
    if path.file_name().is_none() {
        bail!("Database path must include a file name");
    }

    // If an entry already exists at the path, reject symlinks and directories
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            bail!("Symlink path is not allowed for database path");
        }
        if meta.is_dir() {
            bail!("Database path points to a directory");
        }
    }

    Ok(())
}


