use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::AppError;

/// Reads a saved page or a catalog, line endings normalized.
pub fn read_file(path: &Path) -> Result<String, AppError> {
    let text = fs::read_to_string(path).map_err(|source| AppError::FileRead {
        path: path.display().to_string(),
        source,
    })?;
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Writes a catalog: indented, accents left as they are, so that it diffs well in git.
pub fn write_json<T: Serialize>(path: &Path, entries: &[T]) -> Result<(), AppError> {
    let body = serde_json::to_string_pretty(entries)? + "\n";
    fs::write(path, body).map_err(|source| AppError::FileWrite {
        path: path.display().to_string(),
        source,
    })
}
