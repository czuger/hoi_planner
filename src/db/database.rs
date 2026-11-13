use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::error::AppError;

/// Opens the SQLite database, creating the file and its folder if needed, and applies pending
/// migrations.
///
/// # Errors
/// Fails when the URL is invalid, the folder cannot be created, the file cannot be opened or a
/// migration fails.
pub async fn connect(database_url: &str) -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let in_memory = database_url.contains(":memory:");
    if !in_memory {
        create_parent_folder(options.get_filename())?;
    }
    // An in-memory database lives in one connection, so the pool must not open a second one.
    let max_connections = if in_memory { 1 } else { 8 };
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// SQLite creates a missing database file but not the folder holding it.
fn create_parent_folder(database_file: &Path) -> Result<(), AppError> {
    match database_file.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => {
            std::fs::create_dir_all(folder).map_err(AppError::DatabaseFolder)
        }
        _ => Ok(()),
    }
}

/// Asks the database a trivial question, to know whether it answers.
///
/// # Errors
/// Fails when the database cannot be read.
pub async fn ping(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
