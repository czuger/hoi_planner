use sqlx::SqlitePool;

/// State shared by every request handler.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool.
    pub pool: SqlitePool,
    /// What the database is called on screen.
    pub database_name: String,
}
