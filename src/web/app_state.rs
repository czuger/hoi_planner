use std::sync::Arc;

use crate::db::Store;
use crate::domain::Catalogs;

/// State shared by every request handler.
#[derive(Clone)]
pub struct AppState {
    /// The database, which holds the plans.
    pub store: Store,
    /// What the database is called on screen.
    pub database_name: String,
    /// The catalogs, read once at startup: they never change while the app runs.
    pub catalogs: Arc<Catalogs>,
}
