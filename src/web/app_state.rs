use std::sync::Arc;

use sqlx::SqlitePool;

use crate::domain::Catalogs;

/// State shared by every request handler.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool, which holds the plans.
    pub pool: SqlitePool,
    /// What the database is called on screen.
    pub database_name: String,
    /// The catalogs, read once at startup: they never change while the app runs.
    pub catalogs: Arc<Catalogs>,
}
